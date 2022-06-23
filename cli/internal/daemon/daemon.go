package daemon

import (
	"context"
	"crypto/sha256"
	"encoding/hex"
	"fmt"
	"io"
	"net"
	"os"
	"sync"
	"time"

	grpc_recovery "github.com/grpc-ecosystem/go-grpc-middleware/recovery"
	"github.com/hashicorp/go-hclog"
	"github.com/mitchellh/cli"
	"github.com/nightlyone/lockfile"
	"github.com/pkg/errors"
	"github.com/spf13/cobra"
	"github.com/vercel/turborepo/cli/internal/config"
	"github.com/vercel/turborepo/cli/internal/daemon/connector"
	"github.com/vercel/turborepo/cli/internal/fs"
	"github.com/vercel/turborepo/cli/internal/server"
	"github.com/vercel/turborepo/cli/internal/signals"
	"github.com/vercel/turborepo/cli/internal/util"
	"google.golang.org/grpc"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

// Command is the wrapper around the daemon command until we port fully to cobra
type Command struct {
	Config        *config.Config
	UI            cli.Ui
	SignalWatcher *signals.Watcher
}

// Run runs the daemon command
func (c *Command) Run(args []string) int {
	cmd := getCmd(c.Config, c.UI, c.SignalWatcher)
	cmd.SetArgs(args)
	err := cmd.Execute()
	if err != nil {
		return 1
	}
	return 0
}

// Help returns information about the `daemon` command
func (c *Command) Help() string {
	cmd := getCmd(c.Config, c.UI, c.SignalWatcher)
	return util.HelpForCobraCmd(cmd)
}

// Synopsis of daemon command
func (c *Command) Synopsis() string {
	cmd := getCmd(c.Config, c.UI, c.SignalWatcher)
	return cmd.Short
}

type daemon struct {
	logger     hclog.Logger
	repoRoot   fs.AbsolutePath
	timeout    time.Duration
	reqCh      chan struct{}
	timedOutCh chan struct{}
	cleanup    sync.Once
}

func getRepoHash(repoRoot fs.AbsolutePath) string {
	pathHash := sha256.Sum256([]byte(repoRoot.ToString()))
	// We grab a substring of the hash because there is a 108-character limit on the length
	// of a filepath for unix domain socket.
	return hex.EncodeToString(pathHash[:])[:16]
}

func getDaemonFileRoot(repoRoot fs.AbsolutePath) fs.AbsolutePath {
	tempDir := fs.TempDir("turbod")
	hexHash := getRepoHash(repoRoot)
	return tempDir.Join(hexHash)
}

func getLogFilePath(repoRoot fs.AbsolutePath) (fs.AbsolutePath, error) {
	hexHash := getRepoHash(repoRoot)
	base := repoRoot.Base()
	logFilename := fmt.Sprintf("%v-%v.log", hexHash, base)

	logsDir := fs.GetTurboDataDir().Join("logs")
	return logsDir.Join(logFilename), nil
}

func getUnixSocket(repoRoot fs.AbsolutePath) fs.AbsolutePath {
	root := getDaemonFileRoot(repoRoot)
	return root.Join("turbod.sock")
}

func getPidFile(repoRoot fs.AbsolutePath) fs.AbsolutePath {
	root := getDaemonFileRoot(repoRoot)
	return root.Join("turbod.pid")
}

// logError logs an error and outputs it to the UI.
func (d *daemon) logError(err error) {
	d.logger.Error("error", err)
}

// we're only appending, and we're creating the file if it doesn't exist.
// we do not need to read the log file.
var _logFileFlags = os.O_WRONLY | os.O_APPEND | os.O_CREATE

func getCmd(config *config.Config, output cli.Ui, signalWatcher *signals.Watcher) *cobra.Command {
	var idleTimeout time.Duration
	cmd := &cobra.Command{
		Use:           "turbo daemon",
		Short:         "Runs turbod",
		SilenceUsage:  true,
		SilenceErrors: true,
		RunE: func(cmd *cobra.Command, args []string) error {
			logFilePath, err := getLogFilePath(config.Cwd)
			if err != nil {
				return err
			}
			if err := logFilePath.EnsureDir(); err != nil {
				return err
			}
			logFile, err := logFilePath.OpenFile(_logFileFlags, 0644)
			if err != nil {
				return err
			}
			defer func() { _ = logFile.Close() }()
			logger := hclog.New(&hclog.LoggerOptions{
				Output: io.MultiWriter(logFile, os.Stdout),
				Level:  hclog.Debug,
				Color:  hclog.ColorOff,
				Name:   "turbod",
			})
			ctx := cmd.Context()
			d := &daemon{
				logger:     logger,
				repoRoot:   config.Cwd,
				timeout:    idleTimeout,
				reqCh:      make(chan struct{}),
				timedOutCh: make(chan struct{}),
			}
			turboServer, err := server.New(d.logger.Named("rpc server"), config.Cwd, config.TurboVersion, logFilePath)
			if err != nil {
				d.logError(err)
				return err
			}
			defer func() { _ = turboServer.Close() }()
			err = d.runTurboServer(ctx, turboServer, signalWatcher)
			if err != nil {
				d.logError(err)
				return err
			}
			return nil
		},
	}
	cmd.Flags().DurationVar(&idleTimeout, "idle-time", 4*time.Hour, "Set the idle timeout for turbod")
	addDaemonSubcommands(cmd, config, output)
	return cmd
}

func addDaemonSubcommands(cmd *cobra.Command, config *config.Config, output cli.Ui) {
	addStatusCmd(cmd, config, output)
	addStartCmd(cmd, config, output)
	addStopCmd(cmd, config, output)
	addRestartCmd(cmd, config, output)
}

var errInactivityTimeout = errors.New("turbod shut down from inactivity")

// tryAcquirePidfileLock attempts to ensure that only one daemon is running from the given pid file path
// at a time. If this process fails to write its PID to the lockfile, it must exit.
func (d *daemon) tryAcquirePidfileLock(pidPath fs.AbsolutePath) (lockfile.Lockfile, error) {
	lockFile, err := lockfile.New(pidPath.ToString())
	if err != nil {
		// lockfile.New should only return an error if it wasn't given an absolute path.
		// We are attempting to use the type system to enforce that we are passing an
		// absolute path. An error here likely means a bug, and we should crash.
		panic(err)
	}
	if err := lockFile.TryLock(); err != nil {
		return "", err
	}
	return lockFile, nil
}

type rpcServer interface {
	Register(grpcServer server.GRPCServer)
}

func (d *daemon) runTurboServer(parentContext context.Context, rpcServer rpcServer, signalWatcher *signals.Watcher) error {
	ctx, cancel := context.WithCancel(parentContext)
	defer cancel()
	pidPath := getPidFile(d.repoRoot)
	if err := pidPath.EnsureDir(); err != nil {
		return err
	}
	lockFile, err := d.tryAcquirePidfileLock(pidPath)
	if err != nil {
		return err
	}
	signalWatcher.AddOnClose(func() {
		d.unlockPid(lockFile)
	})
	panicHandler := func(thePanic interface{}) error {
		cancel()
		d.logger.Error(fmt.Sprintf("Caught panic %v", thePanic))
		return status.Error(codes.Internal, "server panicked")
	}
	defer d.unlockPid(lockFile)
	// If we have the lock, assume that we are the owners of the socket file,
	// whether it already exists or not. That means we are free to remove it.
	sockPath := getUnixSocket(d.repoRoot)
	if err := sockPath.Remove(); err != nil && !errors.Is(err, os.ErrNotExist) {
		return err
	}
	d.logger.Debug(fmt.Sprintf("Using socket path %v (%v)\n", sockPath, len(sockPath)))
	lis, err := net.Listen("unix", sockPath.ToString())
	if err != nil {
		return err
	}
	// We don't need to explicitly close 'lis', the grpc server will handle that
	s := grpc.NewServer(
		grpc.ChainUnaryInterceptor(
			d.onRequest,
			grpc_recovery.UnaryServerInterceptor(grpc_recovery.WithRecoveryHandler(panicHandler)),
		),
	)
	signalWatcher.AddOnClose(s.GracefulStop)
	go d.timeoutLoop(ctx)

	rpcServer.Register(s)
	errCh := make(chan error)
	go func(errCh chan<- error) {
		if err := s.Serve(lis); err != nil {
			errCh <- err
		}
		close(errCh)
	}(errCh)

	var exitErr error
	select {
	case err, ok := <-errCh:
		if ok {
			exitErr = err
		}
	case <-d.timedOutCh:
		exitErr = errInactivityTimeout
		s.GracefulStop()
	case <-ctx.Done():
		s.GracefulStop()
	}
	// Wait for the server to exit, if it hasn't already.
	// When it does, this channel will close. We don't
	// care about the error in this scenario because we've
	// either requested a close via cancelling the context
	// or an inactivity timeout.
	for range errCh {
	}
	return exitErr
}

func (d *daemon) unlockPid(lockFile lockfile.Lockfile) {
	d.cleanup.Do(func() {
		if err := lockFile.Unlock(); err != nil {
			d.logError(errors.Wrapf(err, "failed unlocking pid file at %v", lockFile))
		}
	})
}

func (d *daemon) onRequest(ctx context.Context, req interface{}, info *grpc.UnaryServerInfo, handler grpc.UnaryHandler) (resp interface{}, err error) {
	d.reqCh <- struct{}{}
	return handler(ctx, req)
}

func (d *daemon) timeoutLoop(ctx context.Context) {
	timeoutCh := time.After(d.timeout)
outer:
	for {
		select {
		case <-d.reqCh:
			timeoutCh = time.After(d.timeout)
		case <-timeoutCh:
			close(d.timedOutCh)
			break outer
		case <-ctx.Done():
			break outer
		}
	}
}

// ClientOpts re-exports connector.Ops to encapsulate the connector package
type ClientOpts = connector.Opts

// Client re-exports connector.Client to encapsulate the connector package
type Client = connector.Client

// GetClient returns a client that can be used to interact with the daemon
func GetClient(ctx context.Context, repoRoot fs.AbsolutePath, logger hclog.Logger, turboVersion string, opts ClientOpts) (*Client, error) {
	sockPath := getUnixSocket(repoRoot)
	pidPath := getPidFile(repoRoot)
	logPath, err := getLogFilePath(repoRoot)
	if err != nil {
		return nil, err
	}
	bin, err := os.Executable()
	if err != nil {
		return nil, err
	}
	c := &connector.Connector{
		Logger:       logger.Named("TurbodClient"),
		Bin:          bin,
		Opts:         opts,
		SockPath:     sockPath,
		PidPath:      pidPath,
		LogPath:      logPath,
		TurboVersion: turboVersion,
	}
	client, err := c.Connect(ctx)
	if err != nil {
		return nil, err
	}
	return client, nil
}

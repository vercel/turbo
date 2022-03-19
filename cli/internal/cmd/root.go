package cmd

import (
	"errors"
	"os"
	"strings"
	"time"

	"github.com/spf13/cobra"
	"github.com/vercel/turborepo/cli/internal/cmd/auth"
	"github.com/vercel/turborepo/cli/internal/cmd/info"
	"github.com/vercel/turborepo/cli/internal/cmd/prune"
	"github.com/vercel/turborepo/cli/internal/cmd/run"
	"github.com/vercel/turborepo/cli/internal/cmdutil"
	"github.com/vercel/turborepo/cli/internal/config"
	"github.com/vercel/turborepo/cli/internal/logger"
	"github.com/vercel/turborepo/cli/internal/process"
)

var rootCmd = &cobra.Command{
	Use:   "turbo <command> [<args>]",
	Short: "Turborepo is a very fast Javascript build tool",
	Long: `The High-performance Build System for JavaScript & TypeScript Codebases.
Complete documentation is available at https://turborepo.com.`,
}

func Execute(version string, processes *process.Manager) int {
	logger := logger.New()

	err := runCmd(logger, version, processes)
	if err == nil {
		return 0
	}

	logger.Printf(err.Error())

	var cmdErr *cmdutil.Error
	if errors.As(err, &cmdErr) {
		return cmdErr.ExitCode
	}

	return 1
}

func runCmd(logger *logger.Logger, version string, processes *process.Manager) error {
	rootCmd.SilenceUsage = true
	rootCmd.SilenceErrors = true
	rootCmd.CompletionOptions.DisableDefaultCmd = true

	rootCmd.Version = version
	rootCmd.SetVersionTemplate(`{{printf "%s" .Version}}
`)

	cfg, err := config.New(logger, version)
	if err != nil {
		return err
	}

	rootCmd.PersistentFlags().CountVarP(&cfg.Level, "level", "l", "set log level")
	rootCmd.PersistentFlags().BoolVar(&cfg.NoColor, "no-color", false, "disable color output")
	rootCmd.PersistentFlags().StringVar(&cfg.Token, "token", cfg.Token, "vercel token")
	rootCmd.PersistentFlags().StringVar(&cfg.TeamSlug, "team", cfg.TeamSlug, "vercel team slug")
	rootCmd.PersistentFlags().StringVar(&cfg.ApiUrl, "api", cfg.ApiUrl, "vercel api url")
	rootCmd.PersistentFlags().StringVar(&cfg.LoginUrl, "url", cfg.LoginUrl, "vercel login url")
	rootCmd.PersistentFlags().BoolVar(&cfg.NoGC, "no-gc", false, "")
	rootCmd.PersistentFlags().StringVar(&cfg.Heap, "heap", "", "outputs the heap trace to the given file")
	rootCmd.PersistentFlags().StringVar(&cfg.Trace, "trace", "", "outputs the cpu trace to the given file")
	rootCmd.PersistentFlags().StringVar(&cfg.CpuProfile, "cpu-profile", "", "outputs the cpu profile to the given file")

	rootCmd.PersistentFlags().Lookup("token").DefValue = ""
	rootCmd.PersistentFlags().Lookup("no-gc").Hidden = true

	ch := &cmdutil.Helper{
		Logger:    logger,
		Config:    cfg,
		Processes: processes,
	}

	rootCmd.PersistentPreRunE = ch.PreRun()

	runCmd := run.RunCmd(ch)
	pruneCmd := prune.PruneCmd(ch)
	if runCmd == nil || pruneCmd == nil {
		return ch.Logger.Errorf("could not determine cwd")
	}

	rootCmd.AddCommand(info.BinCmd(ch))
	rootCmd.AddCommand(auth.LinkCmd(ch))
	rootCmd.AddCommand(auth.UnlinkCmd(ch))
	rootCmd.AddCommand(auth.LoginCmd(ch))
	rootCmd.AddCommand(auth.LogoutCmd(ch))
	rootCmd.AddCommand(runCmd)
	rootCmd.AddCommand(pruneCmd)

	cpuProfile := false
	for _, arg := range os.Args {
		if strings.Contains(arg, "cpu-profile") {
			cpuProfile = true
			break
		}
	}
	if cpuProfile {
		// The CPU profiler in Go only runs at 100 Hz, which is far too slow to
		// return useful information for esbuild, since it's so fast. Let's keep
		// running for 30 seconds straight, which should give us 3,000 samples.
		seconds := 30.0
		start := time.Now()
		for time.Since(start).Seconds() < seconds {
			rootCmd.Execute()
		}
	}

	return rootCmd.Execute()
}

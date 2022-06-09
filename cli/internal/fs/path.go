package fs

import (
	"fmt"
	iofs "io/fs"
	"io/ioutil"
	"os"
	"path/filepath"
	"reflect"

	"github.com/spf13/pflag"
)

// AbsolutePath represents a platform-dependent absolute path on the filesystem,
// and is used to enfore correct path manipulation
type AbsolutePath string

func CheckedToAbsolutePath(s string) (AbsolutePath, error) {
	if filepath.IsAbs(s) {
		return AbsolutePath(s), nil
	}
	return "", fmt.Errorf("%v is not an absolute path", s)
}

// ResolveUnknownPath returns unknown if it is an absolute path, otherwise, it
// assumes unknown is a path relative to the given root.
func ResolveUnknownPath(root AbsolutePath, unknown string) AbsolutePath {
	if filepath.IsAbs(unknown) {
		return AbsolutePath(unknown)
	}
	return root.Join(unknown)
}

func UnsafeToAbsolutePath(s string) AbsolutePath {
	return AbsolutePath(s)
}

func GetCwd() (AbsolutePath, error) {
	cwdRaw, err := os.Getwd()
	if err != nil {
		return "", fmt.Errorf("invalid working directory: %w", err)
	}
	// We evaluate symlinks here because the package managers
	// we support do the same.
	cwdRaw, err = filepath.EvalSymlinks(cwdRaw)
	if err != nil {
		return "", fmt.Errorf("evaluating symlinks in cwd: %w", err)
	}
	cwd, err := CheckedToAbsolutePath(cwdRaw)
	if err != nil {
		return "", fmt.Errorf("cwd is not an absolute path %v: %v", cwdRaw, err)
	}
	return cwd, nil
}

func (ap AbsolutePath) ToStringDuringMigration() string {
	return ap.asString()
}

func (ap AbsolutePath) Join(args ...string) AbsolutePath {
	return AbsolutePath(filepath.Join(ap.asString(), filepath.Join(args...)))
}
func (ap AbsolutePath) asString() string {
	return string(ap)
}
func (ap AbsolutePath) Dir() AbsolutePath {
	return AbsolutePath(filepath.Dir(ap.asString()))
}

// MkdirAll is the AbsolutePath wrapper for os.MkdirAll
func (ap AbsolutePath) MkdirAll() error {
	return os.MkdirAll(ap.asString(), DirPermissions|0644)
}
func (ap AbsolutePath) Open() (*os.File, error) {
	return os.Open(ap.asString())
}

func (ap AbsolutePath) FileExists() bool {
	return FileExists(ap.asString())
}

// ReadFile reads the contents of the specified file
func (ap AbsolutePath) ReadFile() ([]byte, error) {
	return ioutil.ReadFile(ap.asString())
}

// WriteFile writes the contents of the specified file
func (ap AbsolutePath) WriteFile(contents []byte, mode os.FileMode) error {
	return ioutil.WriteFile(ap.asString(), contents, mode)
}

// EnsureDir ensures that the directory containing this file exists
func (ap AbsolutePath) EnsureDir() error {
	return EnsureDir(ap.asString())
}

// Create is the AbsolutePath wrapper for os.Create
func (ap AbsolutePath) Create() (*os.File, error) {
	return os.Create(ap.asString())
}

// Ext implements filepath.Ext for an absolute path
func (ap AbsolutePath) Ext() string {
	return filepath.Ext(ap.asString())
}

// ToString returns the string representation of this absolute path. Used for
// interfacing with APIs that require a string
func (ap AbsolutePath) ToString() string {
	return ap.asString()
}

// RelativePathString returns the relative path from this AbsolutePath to another absolute path in string form as a string
func (ap AbsolutePath) RelativePathString(path string) (string, error) {
	return filepath.Rel(ap.asString(), path)
}

// Remove removes the file or (empty) directory at the given path
func (ap AbsolutePath) Remove() error {
	return os.Remove(ap.asString())
}

// RenameTo implements os.Rename for an absolute path
func (ap AbsolutePath) RenameTo(target AbsolutePath) error {
	return os.Rename(ap.asString(), target.asString())
}

// GetVolumeRoot returns the root directory given an absolute path.
func GetVolumeRoot(absolutePath string) string {
	return filepath.VolumeName(absolutePath) + string(os.PathSeparator)
}

// CreateDirFSAtRoot creates an `os.dirFS` instance at the root of the
// volume containing the specified path.
func CreateDirFSAtRoot(absolutePath string) iofs.FS {
	return os.DirFS(GetVolumeRoot(absolutePath))
}

// GetDirFSRootPath returns the root path of a os.dirFS.
func GetDirFSRootPath(fsys iofs.FS) string {
	// We can't typecheck fsys to enforce using an `os.dirFS` because the
	// type isn't exported from `os`. So instead, reflection. 🤷‍♂️

	fsysType := reflect.TypeOf(fsys).Name()
	if fsysType != "dirFS" {
		// This is not a user error, fail fast
		panic("GetDirFSRootPath must receive an os.dirFS")
	}

	// The underlying type is a string; this is the original path passed in.
	return reflect.ValueOf(fsys).String()
}

// IofsRelativePath calculates a `os.dirFS`-friendly path from an absolute system path.
func IofsRelativePath(fsysRoot string, absolutePath string) (string, error) {
	return filepath.Rel(fsysRoot, absolutePath)
}

type pathValue struct {
	base     AbsolutePath
	current  *AbsolutePath
	defValue string
}

func (pv *pathValue) String() string {
	if *pv.current == "" {
		return ResolveUnknownPath(pv.base, pv.defValue).ToString()
	}
	return pv.current.ToString()
}

func (pv *pathValue) Set(value string) error {
	*pv.current = ResolveUnknownPath(pv.base, value)
	return nil
}

func (pv *pathValue) Type() string {
	return "path"
}

var _ pflag.Value = &pathValue{}

// AbsolutePathVar adds a flag interpreted as an absolute path to the given FlagSet.
// It currently requires a root because relative paths are interpreted relative to the
// given root.
func AbsolutePathVar(flags *pflag.FlagSet, target *AbsolutePath, name string, root AbsolutePath, usage string, defValue string) {
	value := &pathValue{
		base:     root,
		current:  target,
		defValue: defValue,
	}
	flags.Var(value, name, usage)
}

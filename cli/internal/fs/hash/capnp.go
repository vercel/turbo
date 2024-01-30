// Package hash contains the capnp schema and hashing functions for the turbo cache
//
// it depends on the generated capnp schema in ./capnp. to regenerate the schema,
// you need the capnp binary as well as capnpc-go available in your path. then run:
//
// capnp compile -I std -ogo proto.capnp
//
// in crates/turborepo-lib/src/hash or run `make turbo-capnp` in the `cli` directory.
package hash

import (
	"encoding/hex"
	"sort"

	capnp "capnproto.org/go/capnp/v3"
	"github.com/vercel/turbo/cli/internal/env"
	turbo_capnp "github.com/vercel/turbo/cli/internal/fs/hash/capnp"
	"github.com/vercel/turbo/cli/internal/lockfile"
	"github.com/vercel/turbo/cli/internal/turbopath"
	"github.com/vercel/turbo/cli/internal/util"
	"github.com/vercel/turbo/cli/internal/xxhash"
)

// TaskHashable is a hashable representation of a task to be run
type TaskHashable struct {
	GlobalHash           string
	TaskDependencyHashes []string
	HashOfFiles          string
	ExternalDepsHash     string

	PackageDir   turbopath.AnchoredUnixPath
	Task         string
	Outputs      TaskOutputs
	PassThruArgs []string

	Env             []string
	ResolvedEnvVars env.EnvironmentVariablePairs
	PassThroughEnv  []string
	EnvMode         util.EnvMode
	DotEnv          turbopath.AnchoredUnixPathArray
}

// GlobalHashable is a hashable representation of global dependencies for tasks
type GlobalHashable struct {
	GlobalCacheKey       string
	GlobalFileHashMap    map[turbopath.AnchoredUnixPath]string
	RootExternalDepsHash string
	Env                  []string
	ResolvedEnvVars      env.EnvironmentVariablePairs
	PassThroughEnv       []string
	EnvMode              util.EnvMode
	FrameworkInference   bool

	// NOTE! This field is _explicitly_ ordered and should not be sorted.
	DotEnv turbopath.AnchoredUnixPathArray
}

// TaskOutputs represents the patterns for including and excluding files from outputs
type TaskOutputs struct {
	Inclusions []string
	Exclusions []string
}

// Sort contents of task outputs
func (to *TaskOutputs) Sort() {
	sort.Strings(to.Inclusions)
	sort.Strings(to.Exclusions)
}

// HashTaskHashable performs the hash for a TaskHashable, using capnproto for stable cross platform / language hashing
//
// NOTE: This function is _explicitly_ ordered and should not be sorted.
//
//		Order is important for the hash, and is as follows:
//		- GlobalHash
//		- PackageDir
//		- HashOfFiles
//		- ExternalDepsHash
//		- Task
//		- EnvMode
//		- Outputs
//		- TaskDependencyHashes
//		- PassThruArgs
//		- Env
//		- PassThroughEnv
//	 - DotEnv
//	 - ResolvedEnvVars
func HashTaskHashable(task *TaskHashable) (string, error) {
	arena := capnp.SingleSegment(nil)

	_, seg, err := capnp.NewMessage(arena)
	if err != nil {
		return "", err
	}

	taskMsg, err := turbo_capnp.NewRootTaskHashable(seg)
	if err != nil {
		return "", err
	}

	err = taskMsg.SetGlobalHash(task.GlobalHash)
	if err != nil {
		return "", err
	}

	err = taskMsg.SetPackageDir(task.PackageDir.ToString())
	if err != nil {
		return "", err
	}

	err = taskMsg.SetHashOfFiles(task.HashOfFiles)
	if err != nil {
		return "", err
	}

	err = taskMsg.SetExternalDepsHash(task.ExternalDepsHash)
	if err != nil {
		return "", err
	}

	err = taskMsg.SetTask(task.Task)
	if err != nil {
		return "", err
	}

	{
		var envMode turbo_capnp.TaskHashable_EnvMode
		switch task.EnvMode {
		case util.Infer:
			panic("task inferred status should have already been resolved")
		case util.Loose:
			envMode = turbo_capnp.TaskHashable_EnvMode_loose
		case util.Strict:
			envMode = turbo_capnp.TaskHashable_EnvMode_strict
		}

		taskMsg.SetEnvMode(envMode)
	}

	{
		deps, err := taskMsg.NewOutputs()
		if err != nil {
			return "", err
		}

		err = assignList(task.Outputs.Inclusions, deps.SetInclusions, seg)
		if err != nil {
			return "", err
		}

		err = assignList(task.Outputs.Exclusions, deps.SetExclusions, seg)
		if err != nil {
			return "", err
		}

		err = taskMsg.SetOutputs(deps)
		if err != nil {
			return "", err
		}
	}

	err = assignList(task.TaskDependencyHashes, taskMsg.SetTaskDependencyHashes, seg)
	if err != nil {
		return "", err
	}

	err = assignList(task.PassThruArgs, taskMsg.SetPassThruArgs, seg)
	if err != nil {
		return "", err
	}

	err = assignList(task.Env, taskMsg.SetEnv, seg)
	if err != nil {
		return "", err
	}

	err = assignList(task.PassThroughEnv, taskMsg.SetPassThruEnv, seg)
	if err != nil {
		return "", err
	}

	err = assignAnchoredUnixArray(task.DotEnv, taskMsg.SetDotEnv, seg)
	if err != nil {
		return "", err
	}

	err = assignList(task.ResolvedEnvVars, taskMsg.SetResolvedEnvVars, seg)
	if err != nil {
		return "", err
	}

	return HashMessage(taskMsg.Message())
}

// HashGlobalHashable performs the hash for a GlobalHashable, using capnproto for stable cross platform / language hashing
//
// NOTE: This function is _explicitly_ ordered and should not be sorted.
//
//			Order is important for the hash, and is as follows:
//			- GlobalCacheKey
//			- GlobalFileHashMap
//			- RootExternalDepsHash
//	   - Env
//	   - ResolvedEnvVars
//	   - PassThroughEnv
//	   - EnvMode
//	   - FrameworkInference
//	   - DotEnv
func HashGlobalHashable(global *GlobalHashable) (string, error) {
	arena := capnp.SingleSegment(nil)

	_, seg, err := capnp.NewMessage(arena)
	if err != nil {
		return "", err
	}

	globalMsg, err := turbo_capnp.NewRootGlobalHashable(seg)
	if err != nil {
		return "", err
	}

	err = globalMsg.SetGlobalCacheKey(global.GlobalCacheKey)
	if err != nil {
		return "", err
	}

	{
		entries, err := globalMsg.NewGlobalFileHashMap(int32(len(global.GlobalFileHashMap)))
		if err != nil {
			return "", err
		}

		err = assignSortedHashMap(global.GlobalFileHashMap, func(i int, key string, value string) error {
			entry := entries.At(i)

			err = entry.SetKey(key)
			if err != nil {
				return err
			}

			err = entry.SetValue(value)
			if err != nil {
				return err
			}

			return nil
		})
		if err != nil {
			return "", err
		}
	}

	err = globalMsg.SetRootExternalDepsHash(global.RootExternalDepsHash)
	if err != nil {
		return "", err
	}

	err = assignList(global.Env, globalMsg.SetEnv, seg)
	if err != nil {
		return "", err
	}

	err = assignList(global.ResolvedEnvVars, globalMsg.SetResolvedEnvVars, seg)
	if err != nil {
		return "", err
	}

	err = assignList(global.PassThroughEnv, globalMsg.SetPassThroughEnv, seg)
	if err != nil {
		return "", err
	}

	{
		var envMode turbo_capnp.GlobalHashable_EnvMode
		switch global.EnvMode {
		case util.Infer:
			envMode = turbo_capnp.GlobalHashable_EnvMode_infer
		case util.Loose:
			envMode = turbo_capnp.GlobalHashable_EnvMode_loose
		case util.Strict:
			envMode = turbo_capnp.GlobalHashable_EnvMode_strict
		}

		globalMsg.SetEnvMode(envMode)
	}

	globalMsg.SetFrameworkInference(global.FrameworkInference)

	err = assignAnchoredUnixArray(global.DotEnv, globalMsg.SetDotEnv, seg)
	if err != nil {
		return "", err
	}

	return HashMessage(globalMsg.Message())
}

// HashLockfilePackages hashes lockfile packages
func HashLockfilePackages(packages []lockfile.Package) (string, error) {
	arena := capnp.SingleSegment(nil)

	_, seg, err := capnp.NewMessage(arena)
	if err != nil {
		return "", err
	}

	globalMsg, err := turbo_capnp.NewRootLockFilePackages(seg)
	if err != nil {
		return "", err
	}

	entries, err := globalMsg.NewPackages(int32(len(packages)))
	if err != nil {
		return "", err
	}
	for i, pkg := range packages {
		entry := entries.At(i)

		err = entry.SetKey(pkg.Key)
		if err != nil {
			return "", err
		}

		// We explicitly write Version to match Rust behavior when writing empty strings
		// The Go library will emit a null pointer if the string is empty instead
		// of a zero length list.
		err = capnp.Struct(entry).SetNewText(1, pkg.Version)
		if err != nil {
			return "", err
		}

		entry.SetFound(pkg.Found)
	}

	return HashMessage(globalMsg.Message())
}

// HashFileHashes hashes files
func HashFileHashes(fileHashes map[turbopath.AnchoredUnixPath]string) (string, error) {
	arena := capnp.SingleSegment(nil)

	_, seg, err := capnp.NewMessage(arena)
	if err != nil {
		return "", err
	}

	globalMsg, err := turbo_capnp.NewRootFileHashes(seg)
	if err != nil {
		return "", err
	}

	{
		entries, err := globalMsg.NewFileHashes(int32(len(fileHashes)))
		if err != nil {
			return "", err
		}

		err = assignSortedHashMap(fileHashes, func(i int, key string, value string) error {
			entry := entries.At(i)

			err = entry.SetKey(key)
			if err != nil {
				return err
			}

			err = entry.SetValue(value)
			if err != nil {
				return err
			}

			return nil
		})
		if err != nil {
			return "", err
		}
	}

	return HashMessage(globalMsg.Message())
}

// HashMessage hashes a capnp message using xxhash
func HashMessage(msg *capnp.Message) (string, error) {
	root, err := msg.Root()
	if err != nil {
		return "", err
	}

	bytes, err := capnp.Canonicalize(root.Struct())
	if err != nil {
		return "", err
	}

	// _ = turbopath.AbsoluteSystemPath(".turbo/go-hash").WriteFile(bytes, 0644)

	digest := xxhash.New()
	_, err = digest.Write(bytes)
	if err != nil {
		return "", err
	}

	out := digest.Sum(nil)

	return hex.EncodeToString(out), nil
}

// assignSortedHashMap gets a list of key value pairs and then sort them by key
// to do this we need three lists, one for the keys, one for the string representation of the keys,
// and one for the indices of the keys
func assignSortedHashMap(packages map[turbopath.AnchoredUnixPath]string, setEntry func(int, string, string) error) error {
	keys := make([]turbopath.AnchoredUnixPath, len(packages))
	keyStrs := make([]string, len(packages))
	keyIndices := make([]int, len(packages))

	i := 0
	for k := range packages {
		keys[i] = k
		keyStrs[i] = k.ToString()
		keyIndices[i] = i
		i++
	}

	sort.Slice(keyIndices, func(i, j int) bool {
		return keyStrs[keyIndices[i]] < keyStrs[keyIndices[j]]
	})

	for i, idx := range keyIndices {
		err := setEntry(i, keyStrs[idx], packages[keys[idx]])
		if err != nil {
			return err
		}
	}

	return nil
}

func assignList(list []string, fn func(capnp.TextList) error, seg *capnp.Segment) error {
	textList, err := capnp.NewTextList(seg, int32(len(list)))
	if err != nil {
		return err
	}
	for i, v := range list {
		err = textList.Set(i, v)
		if err != nil {
			return err
		}
	}
	return fn(textList)
}

func assignAnchoredUnixArray(paths turbopath.AnchoredUnixPathArray, fn func(capnp.TextList) error, seg *capnp.Segment) error {
	textList, err := capnp.NewTextList(seg, int32(len(paths)))
	if err != nil {
		return err
	}
	for i, v := range paths {
		err = textList.Set(i, v.ToString())
		if err != nil {
			return err
		}
	}
	return fn(textList)
}

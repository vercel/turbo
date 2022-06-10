// Adapted from https://github.com/thought-machine/please
// Copyright Thought Machine, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
package fs

import (
	"errors"
	"os"
	"path/filepath"

	"github.com/karrick/godirwalk"
)

// RecursiveCopy copies either a single file or a directory.
// 'mode' is the mode of the destination file.
func RecursiveCopy(from string, to string, mode os.FileMode) error {
	info, err := os.Lstat(from)
	if err != nil {
		return err
	}

	// We need to know if it is a symlink to a directory since we actually
	// resolve all things now instead of persisting the link.
	//
	// We can't simply switch to os.Stat above without throwing errors in
	// places where we didn't previously.
	isSymlink := err == nil && info.Mode()&os.ModeSymlink == os.ModeSymlink
	isSymlinkToDir := false
	if isSymlink {
		// We intentionally do not error on broken symlinks.
		info, _ := os.Stat(from)
		isSymlinkToDir = info.IsDir()
	}

	isDir := info.IsDir() || isSymlinkToDir

	if isDir {
		return WalkMode(from, func(name string, isDir bool, fileMode os.FileMode) error {
			dest := filepath.Join(to, name[len(from):])
			if isDir {
				return os.MkdirAll(dest, DirPermissions)
			}
			return CopyFile(name, dest, mode)
		})
	}
	return CopyFile(from, to, mode)
}

// Walk implements an equivalent to filepath.Walk.
// It's implemented over github.com/karrick/godirwalk but the provided interface doesn't use that
// to make it a little easier to handle.
func Walk(rootPath string, callback func(name string, isDir bool) error) error {
	return WalkMode(rootPath, func(name string, isDir bool, mode os.FileMode) error {
		return callback(name, isDir)
	})
}

// WalkMode is like Walk but the callback receives an additional type specifying the file mode type.
// N.B. This only includes the bits of the mode that determine the mode type, not the permissions.
func WalkMode(rootPath string, callback func(name string, isDir bool, mode os.FileMode) error) error {
	return godirwalk.Walk(rootPath, &godirwalk.Options{
		Callback: func(name string, info *godirwalk.Dirent) error {
			// currently we support symlinked files, but not symlinked directories:
			// For copying, we Mkdir and bail if we encounter a symlink to a directoy
			// For finding packages, we enumerate the symlink, but don't follow inside
			isDir, err := info.IsDirOrSymlinkToDir()
			if err != nil {
				pathErr := &os.PathError{}
				if errors.As(err, &pathErr) {
					// If we have a broken link, skip this entry
					return godirwalk.SkipThis
				}
				return err
			}
			return callback(name, isDir, info.ModeType())
		},
		ErrorCallback: func(pathname string, err error) godirwalk.ErrorAction {
			pathErr := &os.PathError{}
			if errors.As(err, &pathErr) {
				return godirwalk.SkipNode
			}
			return godirwalk.Halt
		},
		Unsorted:            true,
		AllowNonDirectory:   true,
		FollowSymbolicLinks: false,
	})
}

package fs

import (
	"testing"

	"github.com/vercel/turbo/cli/internal/turbopath"
	"gotest.tools/v3/assert"
)

func Test_RecursiveCopyBadSrc(t *testing.T) {
	src := turbopath.AbsoluteSystemPath("foobar")
	dst := turbopath.AbsoluteSystemPath("/tmp/foobar")
	for i := 0; i < 1_000_000_000; i++ {
		err := RecursiveCopy(src, dst)
		assert.ErrorContains(t, err, "Path is not absolute: foobar")
	}
}

func Test_RecursiveCopyBadDst(t *testing.T) {
	src := turbopath.AbsoluteSystemPath("/tmp/foobar")
	dst := turbopath.AbsoluteSystemPath("foobar")
	for i := 0; i < 1_000_000_000; i++ {
		err := RecursiveCopy(src, dst)
		assert.ErrorContains(t, err, "Path is not absolute: foobar")
	}
}

func Test_RecursiveCopyMissingFile(t *testing.T) {
	base := turbopath.AbsoluteSystemPath(t.TempDir())
	for i := 0; i < 1_000_000_000; i++ {
		err := RecursiveCopy(base.UntypedJoin("src"), base.UntypedJoin("dst"))
		assert.ErrorContains(t, err, "IO Error No such file or directory (os error 2)")
	}
}

func Test_RecursiveCopyCopiesFiles(t *testing.T) {
	base := turbopath.AbsoluteSystemPath(t.TempDir())
	src := base.UntypedJoin("src")
	err := src.Mkdir(0775)
	assert.NilError(t, err, "mkdir")
	for i := 0; i < 1_000_000_000; i++ {
		err = RecursiveCopy(src, base.UntypedJoin("dst"))
		assert.NilError(t, err, "recursive copy")
	}
}

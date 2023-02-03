package ffi

// #include "bindings.h"
//
// #cgo LDFLAGS: -L${SRCDIR} -lturborepo_ffi
// #cgo windows LDFLAGS: -lole32 -lbcrypt -lws2_32 -luserenv
import "C"

import (
	"errors"
	"reflect"
	"unsafe"

	ffi_proto "github.com/vercel/turbo/cli/internal/ffi/proto"
	"google.golang.org/protobuf/proto"
)

// Unmarshal consumes a buffer and parses it into a proto.Message
func Unmarshal[M proto.Message](b C.Buffer, c M) error {
	bytes := toBytes(b)
	if err := proto.Unmarshal(bytes, c); err != nil {
		return err
	}

	b.Free()

	return nil
}

// Marshal consumes a proto.Message and returns a bufferfire
//
// NOTE: the buffer must be freed by calling `Free` on it
func Marshal[M proto.Message](c M) C.Buffer {
	bytes, err := proto.Marshal(c)
	if err != nil {
		panic(err)
	}

	return toBuffer(bytes)
}

func (c C.Buffer) Free() {
	C.free(unsafe.Pointer(c.data))
}

// rather than use C.GoBytes, we use this function to avoid copying the bytes,
// since it is going to be immediately Unmarshalled into a proto.Message
func toBytes(b C.Buffer) []byte {
	var out []byte

	len := (uint32)(b.len)

	sh := (*reflect.SliceHeader)(unsafe.Pointer(&out))
	sh.Data = uintptr(unsafe.Pointer(b.data))
	sh.Len = int(len)
	sh.Cap = int(len)

	return out
}

func toBuffer(bytes []byte) C.Buffer {
	b := C.Buffer{}
	b.len = C.uint(len(bytes))
	b.data = (*C.uchar)(C.CBytes(bytes))
	return b
}

// GetTurboDataDir returns the path to the Turbo data directory
func GetTurboDataDir() string {
	buffer := C.get_turbo_data_dir()
	resp := ffi_proto.TurboDataDirResp{}
	if err := Unmarshal(buffer, resp.ProtoReflect().Interface()); err != nil {
		panic(err)
	}
	return resp.Dir
}

// NpmTransitiveDeps returns the transitive external deps of a given package based on the deps and specifiers given
func NpmTransitiveDeps(content []byte, pkgDir string, unresolvedDeps map[string]string) ([]*ffi_proto.LockfilePackage, error) {
	req := ffi_proto.TransitiveDepsRequest{
		Contents:       content,
		WorkspaceDir:   pkgDir,
		UnresolvedDeps: unresolvedDeps,
	}
	reqBuf := Marshal(&req)
	resBuf := C.npm_transitive_closure(reqBuf)
	reqBuf.Free()

	resp := ffi_proto.TransitiveDepsResponse{}
	if err := Unmarshal(resBuf, resp.ProtoReflect().Interface()); err != nil {
		panic(err)
	}

	if err := resp.GetError(); err != "" {
		return nil, errors.New(err)
	}

	list := resp.GetPackages()
	return list.GetList(), nil
}

func NpmSubgraph(content []byte, workspaces []string, packages []string) ([]byte, error) {
	req := ffi_proto.SubgraphRequest{
		Contents:   content,
		Workspaces: workspaces,
		Packages:   packages,
	}
	reqBuf := Marshal(&req)
	resBuf := C.npm_subgraph(reqBuf)
	reqBuf.Free()

	resp := ffi_proto.SubgraphResponse{}
	if err := Unmarshal(resBuf, resp.ProtoReflect().Interface()); err != nil {
		panic(err)
	}

	if err := resp.GetError(); err != "" {
		return nil, errors.New(err)
	}

	return resp.GetContents(), nil
}

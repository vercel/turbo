// Code generated by protoc-gen-go. DO NOT EDIT.
// versions:
// 	protoc-gen-go v1.28.0
// 	protoc        v3.21.9
// source: messages.proto

package proto

import (
	protoreflect "google.golang.org/protobuf/reflect/protoreflect"
	protoimpl "google.golang.org/protobuf/runtime/protoimpl"
	reflect "reflect"
	sync "sync"
)

const (
	// Verify that this generated code is sufficiently up-to-date.
	_ = protoimpl.EnforceVersion(20 - protoimpl.MinVersion)
	// Verify that runtime/protoimpl is sufficiently up-to-date.
	_ = protoimpl.EnforceVersion(protoimpl.MaxVersion - 20)
)

type TurboDataDirResp struct {
	state         protoimpl.MessageState
	sizeCache     protoimpl.SizeCache
	unknownFields protoimpl.UnknownFields

	Dir string `protobuf:"bytes,1,opt,name=dir,proto3" json:"dir,omitempty"`
}

func (x *TurboDataDirResp) Reset() {
	*x = TurboDataDirResp{}
	if protoimpl.UnsafeEnabled {
		mi := &file_messages_proto_msgTypes[0]
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		ms.StoreMessageInfo(mi)
	}
}

func (x *TurboDataDirResp) String() string {
	return protoimpl.X.MessageStringOf(x)
}

func (*TurboDataDirResp) ProtoMessage() {}

func (x *TurboDataDirResp) ProtoReflect() protoreflect.Message {
	mi := &file_messages_proto_msgTypes[0]
	if protoimpl.UnsafeEnabled && x != nil {
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		if ms.LoadMessageInfo() == nil {
			ms.StoreMessageInfo(mi)
		}
		return ms
	}
	return mi.MessageOf(x)
}

// Deprecated: Use TurboDataDirResp.ProtoReflect.Descriptor instead.
func (*TurboDataDirResp) Descriptor() ([]byte, []int) {
	return file_messages_proto_rawDescGZIP(), []int{0}
}

func (x *TurboDataDirResp) GetDir() string {
	if x != nil {
		return x.Dir
	}
	return ""
}

type GlobReq struct {
	state         protoimpl.MessageState
	sizeCache     protoimpl.SizeCache
	unknownFields protoimpl.UnknownFields

	BasePath        string   `protobuf:"bytes,1,opt,name=base_path,json=basePath,proto3" json:"base_path,omitempty"`
	IncludePatterns []string `protobuf:"bytes,2,rep,name=include_patterns,json=includePatterns,proto3" json:"include_patterns,omitempty"`
	ExcludePatterns []string `protobuf:"bytes,3,rep,name=exclude_patterns,json=excludePatterns,proto3" json:"exclude_patterns,omitempty"`
	FilesOnly       bool     `protobuf:"varint,4,opt,name=files_only,json=filesOnly,proto3" json:"files_only,omitempty"` // note that the default for a bool is false
}

func (x *GlobReq) Reset() {
	*x = GlobReq{}
	if protoimpl.UnsafeEnabled {
		mi := &file_messages_proto_msgTypes[1]
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		ms.StoreMessageInfo(mi)
	}
}

func (x *GlobReq) String() string {
	return protoimpl.X.MessageStringOf(x)
}

func (*GlobReq) ProtoMessage() {}

func (x *GlobReq) ProtoReflect() protoreflect.Message {
	mi := &file_messages_proto_msgTypes[1]
	if protoimpl.UnsafeEnabled && x != nil {
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		if ms.LoadMessageInfo() == nil {
			ms.StoreMessageInfo(mi)
		}
		return ms
	}
	return mi.MessageOf(x)
}

// Deprecated: Use GlobReq.ProtoReflect.Descriptor instead.
func (*GlobReq) Descriptor() ([]byte, []int) {
	return file_messages_proto_rawDescGZIP(), []int{1}
}

func (x *GlobReq) GetBasePath() string {
	if x != nil {
		return x.BasePath
	}
	return ""
}

func (x *GlobReq) GetIncludePatterns() []string {
	if x != nil {
		return x.IncludePatterns
	}
	return nil
}

func (x *GlobReq) GetExcludePatterns() []string {
	if x != nil {
		return x.ExcludePatterns
	}
	return nil
}

func (x *GlobReq) GetFilesOnly() bool {
	if x != nil {
		return x.FilesOnly
	}
	return false
}

type GlobResp struct {
	state         protoimpl.MessageState
	sizeCache     protoimpl.SizeCache
	unknownFields protoimpl.UnknownFields

	// Types that are assignable to Response:
	//	*GlobResp_Files
	//	*GlobResp_Error
	Response isGlobResp_Response `protobuf_oneof:"response"`
}

func (x *GlobResp) Reset() {
	*x = GlobResp{}
	if protoimpl.UnsafeEnabled {
		mi := &file_messages_proto_msgTypes[2]
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		ms.StoreMessageInfo(mi)
	}
}

func (x *GlobResp) String() string {
	return protoimpl.X.MessageStringOf(x)
}

func (*GlobResp) ProtoMessage() {}

func (x *GlobResp) ProtoReflect() protoreflect.Message {
	mi := &file_messages_proto_msgTypes[2]
	if protoimpl.UnsafeEnabled && x != nil {
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		if ms.LoadMessageInfo() == nil {
			ms.StoreMessageInfo(mi)
		}
		return ms
	}
	return mi.MessageOf(x)
}

// Deprecated: Use GlobResp.ProtoReflect.Descriptor instead.
func (*GlobResp) Descriptor() ([]byte, []int) {
	return file_messages_proto_rawDescGZIP(), []int{2}
}

func (m *GlobResp) GetResponse() isGlobResp_Response {
	if m != nil {
		return m.Response
	}
	return nil
}

func (x *GlobResp) GetFiles() *GlobRespList {
	if x, ok := x.GetResponse().(*GlobResp_Files); ok {
		return x.Files
	}
	return nil
}

func (x *GlobResp) GetError() string {
	if x, ok := x.GetResponse().(*GlobResp_Error); ok {
		return x.Error
	}
	return ""
}

type isGlobResp_Response interface {
	isGlobResp_Response()
}

type GlobResp_Files struct {
	Files *GlobRespList `protobuf:"bytes,1,opt,name=files,proto3,oneof"`
}

type GlobResp_Error struct {
	Error string `protobuf:"bytes,2,opt,name=error,proto3,oneof"`
}

func (*GlobResp_Files) isGlobResp_Response() {}

func (*GlobResp_Error) isGlobResp_Response() {}

type GlobRespList struct {
	state         protoimpl.MessageState
	sizeCache     protoimpl.SizeCache
	unknownFields protoimpl.UnknownFields

	Files []string `protobuf:"bytes,1,rep,name=files,proto3" json:"files,omitempty"`
}

func (x *GlobRespList) Reset() {
	*x = GlobRespList{}
	if protoimpl.UnsafeEnabled {
		mi := &file_messages_proto_msgTypes[3]
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		ms.StoreMessageInfo(mi)
	}
}

func (x *GlobRespList) String() string {
	return protoimpl.X.MessageStringOf(x)
}

func (*GlobRespList) ProtoMessage() {}

func (x *GlobRespList) ProtoReflect() protoreflect.Message {
	mi := &file_messages_proto_msgTypes[3]
	if protoimpl.UnsafeEnabled && x != nil {
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		if ms.LoadMessageInfo() == nil {
			ms.StoreMessageInfo(mi)
		}
		return ms
	}
	return mi.MessageOf(x)
}

// Deprecated: Use GlobRespList.ProtoReflect.Descriptor instead.
func (*GlobRespList) Descriptor() ([]byte, []int) {
	return file_messages_proto_rawDescGZIP(), []int{3}
}

func (x *GlobRespList) GetFiles() []string {
	if x != nil {
		return x.Files
	}
	return nil
}

type TransitiveDepsRequest struct {
	state         protoimpl.MessageState
	sizeCache     protoimpl.SizeCache
	unknownFields protoimpl.UnknownFields

	Contents       []byte            `protobuf:"bytes,1,opt,name=contents,proto3" json:"contents,omitempty"`
	WorkspaceDir   string            `protobuf:"bytes,2,opt,name=workspace_dir,json=workspaceDir,proto3" json:"workspace_dir,omitempty"`
	UnresolvedDeps map[string]string `protobuf:"bytes,3,rep,name=unresolved_deps,json=unresolvedDeps,proto3" json:"unresolved_deps,omitempty" protobuf_key:"bytes,1,opt,name=key,proto3" protobuf_val:"bytes,2,opt,name=value,proto3"`
}

func (x *TransitiveDepsRequest) Reset() {
	*x = TransitiveDepsRequest{}
	if protoimpl.UnsafeEnabled {
		mi := &file_messages_proto_msgTypes[4]
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		ms.StoreMessageInfo(mi)
	}
}

func (x *TransitiveDepsRequest) String() string {
	return protoimpl.X.MessageStringOf(x)
}

func (*TransitiveDepsRequest) ProtoMessage() {}

func (x *TransitiveDepsRequest) ProtoReflect() protoreflect.Message {
	mi := &file_messages_proto_msgTypes[4]
	if protoimpl.UnsafeEnabled && x != nil {
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		if ms.LoadMessageInfo() == nil {
			ms.StoreMessageInfo(mi)
		}
		return ms
	}
	return mi.MessageOf(x)
}

// Deprecated: Use TransitiveDepsRequest.ProtoReflect.Descriptor instead.
func (*TransitiveDepsRequest) Descriptor() ([]byte, []int) {
	return file_messages_proto_rawDescGZIP(), []int{4}
}

func (x *TransitiveDepsRequest) GetContents() []byte {
	if x != nil {
		return x.Contents
	}
	return nil
}

func (x *TransitiveDepsRequest) GetWorkspaceDir() string {
	if x != nil {
		return x.WorkspaceDir
	}
	return ""
}

func (x *TransitiveDepsRequest) GetUnresolvedDeps() map[string]string {
	if x != nil {
		return x.UnresolvedDeps
	}
	return nil
}

type TransitiveDepsResponse struct {
	state         protoimpl.MessageState
	sizeCache     protoimpl.SizeCache
	unknownFields protoimpl.UnknownFields

	// Types that are assignable to Response:
	//	*TransitiveDepsResponse_Packages
	//	*TransitiveDepsResponse_Error
	Response isTransitiveDepsResponse_Response `protobuf_oneof:"response"`
}

func (x *TransitiveDepsResponse) Reset() {
	*x = TransitiveDepsResponse{}
	if protoimpl.UnsafeEnabled {
		mi := &file_messages_proto_msgTypes[5]
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		ms.StoreMessageInfo(mi)
	}
}

func (x *TransitiveDepsResponse) String() string {
	return protoimpl.X.MessageStringOf(x)
}

func (*TransitiveDepsResponse) ProtoMessage() {}

func (x *TransitiveDepsResponse) ProtoReflect() protoreflect.Message {
	mi := &file_messages_proto_msgTypes[5]
	if protoimpl.UnsafeEnabled && x != nil {
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		if ms.LoadMessageInfo() == nil {
			ms.StoreMessageInfo(mi)
		}
		return ms
	}
	return mi.MessageOf(x)
}

// Deprecated: Use TransitiveDepsResponse.ProtoReflect.Descriptor instead.
func (*TransitiveDepsResponse) Descriptor() ([]byte, []int) {
	return file_messages_proto_rawDescGZIP(), []int{5}
}

func (m *TransitiveDepsResponse) GetResponse() isTransitiveDepsResponse_Response {
	if m != nil {
		return m.Response
	}
	return nil
}

func (x *TransitiveDepsResponse) GetPackages() *LockfilePackageList {
	if x, ok := x.GetResponse().(*TransitiveDepsResponse_Packages); ok {
		return x.Packages
	}
	return nil
}

func (x *TransitiveDepsResponse) GetError() string {
	if x, ok := x.GetResponse().(*TransitiveDepsResponse_Error); ok {
		return x.Error
	}
	return ""
}

type isTransitiveDepsResponse_Response interface {
	isTransitiveDepsResponse_Response()
}

type TransitiveDepsResponse_Packages struct {
	Packages *LockfilePackageList `protobuf:"bytes,1,opt,name=packages,proto3,oneof"`
}

type TransitiveDepsResponse_Error struct {
	Error string `protobuf:"bytes,2,opt,name=error,proto3,oneof"`
}

func (*TransitiveDepsResponse_Packages) isTransitiveDepsResponse_Response() {}

func (*TransitiveDepsResponse_Error) isTransitiveDepsResponse_Response() {}

type LockfilePackage struct {
	state         protoimpl.MessageState
	sizeCache     protoimpl.SizeCache
	unknownFields protoimpl.UnknownFields

	Key     string `protobuf:"bytes,1,opt,name=key,proto3" json:"key,omitempty"`
	Version string `protobuf:"bytes,2,opt,name=version,proto3" json:"version,omitempty"`
	Found   bool   `protobuf:"varint,3,opt,name=found,proto3" json:"found,omitempty"`
}

func (x *LockfilePackage) Reset() {
	*x = LockfilePackage{}
	if protoimpl.UnsafeEnabled {
		mi := &file_messages_proto_msgTypes[6]
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		ms.StoreMessageInfo(mi)
	}
}

func (x *LockfilePackage) String() string {
	return protoimpl.X.MessageStringOf(x)
}

func (*LockfilePackage) ProtoMessage() {}

func (x *LockfilePackage) ProtoReflect() protoreflect.Message {
	mi := &file_messages_proto_msgTypes[6]
	if protoimpl.UnsafeEnabled && x != nil {
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		if ms.LoadMessageInfo() == nil {
			ms.StoreMessageInfo(mi)
		}
		return ms
	}
	return mi.MessageOf(x)
}

// Deprecated: Use LockfilePackage.ProtoReflect.Descriptor instead.
func (*LockfilePackage) Descriptor() ([]byte, []int) {
	return file_messages_proto_rawDescGZIP(), []int{6}
}

func (x *LockfilePackage) GetKey() string {
	if x != nil {
		return x.Key
	}
	return ""
}

func (x *LockfilePackage) GetVersion() string {
	if x != nil {
		return x.Version
	}
	return ""
}

func (x *LockfilePackage) GetFound() bool {
	if x != nil {
		return x.Found
	}
	return false
}

type LockfilePackageList struct {
	state         protoimpl.MessageState
	sizeCache     protoimpl.SizeCache
	unknownFields protoimpl.UnknownFields

	List []*LockfilePackage `protobuf:"bytes,1,rep,name=list,proto3" json:"list,omitempty"`
}

func (x *LockfilePackageList) Reset() {
	*x = LockfilePackageList{}
	if protoimpl.UnsafeEnabled {
		mi := &file_messages_proto_msgTypes[7]
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		ms.StoreMessageInfo(mi)
	}
}

func (x *LockfilePackageList) String() string {
	return protoimpl.X.MessageStringOf(x)
}

func (*LockfilePackageList) ProtoMessage() {}

func (x *LockfilePackageList) ProtoReflect() protoreflect.Message {
	mi := &file_messages_proto_msgTypes[7]
	if protoimpl.UnsafeEnabled && x != nil {
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		if ms.LoadMessageInfo() == nil {
			ms.StoreMessageInfo(mi)
		}
		return ms
	}
	return mi.MessageOf(x)
}

// Deprecated: Use LockfilePackageList.ProtoReflect.Descriptor instead.
func (*LockfilePackageList) Descriptor() ([]byte, []int) {
	return file_messages_proto_rawDescGZIP(), []int{7}
}

func (x *LockfilePackageList) GetList() []*LockfilePackage {
	if x != nil {
		return x.List
	}
	return nil
}

type SubgraphRequest struct {
	state         protoimpl.MessageState
	sizeCache     protoimpl.SizeCache
	unknownFields protoimpl.UnknownFields

	Contents   []byte   `protobuf:"bytes,1,opt,name=contents,proto3" json:"contents,omitempty"`
	Workspaces []string `protobuf:"bytes,2,rep,name=workspaces,proto3" json:"workspaces,omitempty"`
	Packages   []string `protobuf:"bytes,3,rep,name=packages,proto3" json:"packages,omitempty"`
}

func (x *SubgraphRequest) Reset() {
	*x = SubgraphRequest{}
	if protoimpl.UnsafeEnabled {
		mi := &file_messages_proto_msgTypes[8]
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		ms.StoreMessageInfo(mi)
	}
}

func (x *SubgraphRequest) String() string {
	return protoimpl.X.MessageStringOf(x)
}

func (*SubgraphRequest) ProtoMessage() {}

func (x *SubgraphRequest) ProtoReflect() protoreflect.Message {
	mi := &file_messages_proto_msgTypes[8]
	if protoimpl.UnsafeEnabled && x != nil {
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		if ms.LoadMessageInfo() == nil {
			ms.StoreMessageInfo(mi)
		}
		return ms
	}
	return mi.MessageOf(x)
}

// Deprecated: Use SubgraphRequest.ProtoReflect.Descriptor instead.
func (*SubgraphRequest) Descriptor() ([]byte, []int) {
	return file_messages_proto_rawDescGZIP(), []int{8}
}

func (x *SubgraphRequest) GetContents() []byte {
	if x != nil {
		return x.Contents
	}
	return nil
}

func (x *SubgraphRequest) GetWorkspaces() []string {
	if x != nil {
		return x.Workspaces
	}
	return nil
}

func (x *SubgraphRequest) GetPackages() []string {
	if x != nil {
		return x.Packages
	}
	return nil
}

type SubgraphResponse struct {
	state         protoimpl.MessageState
	sizeCache     protoimpl.SizeCache
	unknownFields protoimpl.UnknownFields

	// Types that are assignable to Response:
	//	*SubgraphResponse_Contents
	//	*SubgraphResponse_Error
	Response isSubgraphResponse_Response `protobuf_oneof:"response"`
}

func (x *SubgraphResponse) Reset() {
	*x = SubgraphResponse{}
	if protoimpl.UnsafeEnabled {
		mi := &file_messages_proto_msgTypes[9]
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		ms.StoreMessageInfo(mi)
	}
}

func (x *SubgraphResponse) String() string {
	return protoimpl.X.MessageStringOf(x)
}

func (*SubgraphResponse) ProtoMessage() {}

func (x *SubgraphResponse) ProtoReflect() protoreflect.Message {
	mi := &file_messages_proto_msgTypes[9]
	if protoimpl.UnsafeEnabled && x != nil {
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		if ms.LoadMessageInfo() == nil {
			ms.StoreMessageInfo(mi)
		}
		return ms
	}
	return mi.MessageOf(x)
}

// Deprecated: Use SubgraphResponse.ProtoReflect.Descriptor instead.
func (*SubgraphResponse) Descriptor() ([]byte, []int) {
	return file_messages_proto_rawDescGZIP(), []int{9}
}

func (m *SubgraphResponse) GetResponse() isSubgraphResponse_Response {
	if m != nil {
		return m.Response
	}
	return nil
}

func (x *SubgraphResponse) GetContents() []byte {
	if x, ok := x.GetResponse().(*SubgraphResponse_Contents); ok {
		return x.Contents
	}
	return nil
}

func (x *SubgraphResponse) GetError() string {
	if x, ok := x.GetResponse().(*SubgraphResponse_Error); ok {
		return x.Error
	}
	return ""
}

type isSubgraphResponse_Response interface {
	isSubgraphResponse_Response()
}

type SubgraphResponse_Contents struct {
	Contents []byte `protobuf:"bytes,1,opt,name=contents,proto3,oneof"`
}

type SubgraphResponse_Error struct {
	Error string `protobuf:"bytes,2,opt,name=error,proto3,oneof"`
}

func (*SubgraphResponse_Contents) isSubgraphResponse_Response() {}

func (*SubgraphResponse_Error) isSubgraphResponse_Response() {}

var File_messages_proto protoreflect.FileDescriptor

var file_messages_proto_rawDesc = []byte{
	0x0a, 0x0e, 0x6d, 0x65, 0x73, 0x73, 0x61, 0x67, 0x65, 0x73, 0x2e, 0x70, 0x72, 0x6f, 0x74, 0x6f,
	0x22, 0x24, 0x0a, 0x10, 0x54, 0x75, 0x72, 0x62, 0x6f, 0x44, 0x61, 0x74, 0x61, 0x44, 0x69, 0x72,
	0x52, 0x65, 0x73, 0x70, 0x12, 0x10, 0x0a, 0x03, 0x64, 0x69, 0x72, 0x18, 0x01, 0x20, 0x01, 0x28,
	0x09, 0x52, 0x03, 0x64, 0x69, 0x72, 0x22, 0x9b, 0x01, 0x0a, 0x07, 0x47, 0x6c, 0x6f, 0x62, 0x52,
	0x65, 0x71, 0x12, 0x1b, 0x0a, 0x09, 0x62, 0x61, 0x73, 0x65, 0x5f, 0x70, 0x61, 0x74, 0x68, 0x18,
	0x01, 0x20, 0x01, 0x28, 0x09, 0x52, 0x08, 0x62, 0x61, 0x73, 0x65, 0x50, 0x61, 0x74, 0x68, 0x12,
	0x29, 0x0a, 0x10, 0x69, 0x6e, 0x63, 0x6c, 0x75, 0x64, 0x65, 0x5f, 0x70, 0x61, 0x74, 0x74, 0x65,
	0x72, 0x6e, 0x73, 0x18, 0x02, 0x20, 0x03, 0x28, 0x09, 0x52, 0x0f, 0x69, 0x6e, 0x63, 0x6c, 0x75,
	0x64, 0x65, 0x50, 0x61, 0x74, 0x74, 0x65, 0x72, 0x6e, 0x73, 0x12, 0x29, 0x0a, 0x10, 0x65, 0x78,
	0x63, 0x6c, 0x75, 0x64, 0x65, 0x5f, 0x70, 0x61, 0x74, 0x74, 0x65, 0x72, 0x6e, 0x73, 0x18, 0x03,
	0x20, 0x03, 0x28, 0x09, 0x52, 0x0f, 0x65, 0x78, 0x63, 0x6c, 0x75, 0x64, 0x65, 0x50, 0x61, 0x74,
	0x74, 0x65, 0x72, 0x6e, 0x73, 0x12, 0x1d, 0x0a, 0x0a, 0x66, 0x69, 0x6c, 0x65, 0x73, 0x5f, 0x6f,
	0x6e, 0x6c, 0x79, 0x18, 0x04, 0x20, 0x01, 0x28, 0x08, 0x52, 0x09, 0x66, 0x69, 0x6c, 0x65, 0x73,
	0x4f, 0x6e, 0x6c, 0x79, 0x22, 0x55, 0x0a, 0x08, 0x47, 0x6c, 0x6f, 0x62, 0x52, 0x65, 0x73, 0x70,
	0x12, 0x25, 0x0a, 0x05, 0x66, 0x69, 0x6c, 0x65, 0x73, 0x18, 0x01, 0x20, 0x01, 0x28, 0x0b, 0x32,
	0x0d, 0x2e, 0x47, 0x6c, 0x6f, 0x62, 0x52, 0x65, 0x73, 0x70, 0x4c, 0x69, 0x73, 0x74, 0x48, 0x00,
	0x52, 0x05, 0x66, 0x69, 0x6c, 0x65, 0x73, 0x12, 0x16, 0x0a, 0x05, 0x65, 0x72, 0x72, 0x6f, 0x72,
	0x18, 0x02, 0x20, 0x01, 0x28, 0x09, 0x48, 0x00, 0x52, 0x05, 0x65, 0x72, 0x72, 0x6f, 0x72, 0x42,
	0x0a, 0x0a, 0x08, 0x72, 0x65, 0x73, 0x70, 0x6f, 0x6e, 0x73, 0x65, 0x22, 0x24, 0x0a, 0x0c, 0x47,
	0x6c, 0x6f, 0x62, 0x52, 0x65, 0x73, 0x70, 0x4c, 0x69, 0x73, 0x74, 0x12, 0x14, 0x0a, 0x05, 0x66,
	0x69, 0x6c, 0x65, 0x73, 0x18, 0x01, 0x20, 0x03, 0x28, 0x09, 0x52, 0x05, 0x66, 0x69, 0x6c, 0x65,
	0x73, 0x22, 0xf0, 0x01, 0x0a, 0x15, 0x54, 0x72, 0x61, 0x6e, 0x73, 0x69, 0x74, 0x69, 0x76, 0x65,
	0x44, 0x65, 0x70, 0x73, 0x52, 0x65, 0x71, 0x75, 0x65, 0x73, 0x74, 0x12, 0x1a, 0x0a, 0x08, 0x63,
	0x6f, 0x6e, 0x74, 0x65, 0x6e, 0x74, 0x73, 0x18, 0x01, 0x20, 0x01, 0x28, 0x0c, 0x52, 0x08, 0x63,
	0x6f, 0x6e, 0x74, 0x65, 0x6e, 0x74, 0x73, 0x12, 0x23, 0x0a, 0x0d, 0x77, 0x6f, 0x72, 0x6b, 0x73,
	0x70, 0x61, 0x63, 0x65, 0x5f, 0x64, 0x69, 0x72, 0x18, 0x02, 0x20, 0x01, 0x28, 0x09, 0x52, 0x0c,
	0x77, 0x6f, 0x72, 0x6b, 0x73, 0x70, 0x61, 0x63, 0x65, 0x44, 0x69, 0x72, 0x12, 0x53, 0x0a, 0x0f,
	0x75, 0x6e, 0x72, 0x65, 0x73, 0x6f, 0x6c, 0x76, 0x65, 0x64, 0x5f, 0x64, 0x65, 0x70, 0x73, 0x18,
	0x03, 0x20, 0x03, 0x28, 0x0b, 0x32, 0x2a, 0x2e, 0x54, 0x72, 0x61, 0x6e, 0x73, 0x69, 0x74, 0x69,
	0x76, 0x65, 0x44, 0x65, 0x70, 0x73, 0x52, 0x65, 0x71, 0x75, 0x65, 0x73, 0x74, 0x2e, 0x55, 0x6e,
	0x72, 0x65, 0x73, 0x6f, 0x6c, 0x76, 0x65, 0x64, 0x44, 0x65, 0x70, 0x73, 0x45, 0x6e, 0x74, 0x72,
	0x79, 0x52, 0x0e, 0x75, 0x6e, 0x72, 0x65, 0x73, 0x6f, 0x6c, 0x76, 0x65, 0x64, 0x44, 0x65, 0x70,
	0x73, 0x1a, 0x41, 0x0a, 0x13, 0x55, 0x6e, 0x72, 0x65, 0x73, 0x6f, 0x6c, 0x76, 0x65, 0x64, 0x44,
	0x65, 0x70, 0x73, 0x45, 0x6e, 0x74, 0x72, 0x79, 0x12, 0x10, 0x0a, 0x03, 0x6b, 0x65, 0x79, 0x18,
	0x01, 0x20, 0x01, 0x28, 0x09, 0x52, 0x03, 0x6b, 0x65, 0x79, 0x12, 0x14, 0x0a, 0x05, 0x76, 0x61,
	0x6c, 0x75, 0x65, 0x18, 0x02, 0x20, 0x01, 0x28, 0x09, 0x52, 0x05, 0x76, 0x61, 0x6c, 0x75, 0x65,
	0x3a, 0x02, 0x38, 0x01, 0x22, 0x70, 0x0a, 0x16, 0x54, 0x72, 0x61, 0x6e, 0x73, 0x69, 0x74, 0x69,
	0x76, 0x65, 0x44, 0x65, 0x70, 0x73, 0x52, 0x65, 0x73, 0x70, 0x6f, 0x6e, 0x73, 0x65, 0x12, 0x32,
	0x0a, 0x08, 0x70, 0x61, 0x63, 0x6b, 0x61, 0x67, 0x65, 0x73, 0x18, 0x01, 0x20, 0x01, 0x28, 0x0b,
	0x32, 0x14, 0x2e, 0x4c, 0x6f, 0x63, 0x6b, 0x66, 0x69, 0x6c, 0x65, 0x50, 0x61, 0x63, 0x6b, 0x61,
	0x67, 0x65, 0x4c, 0x69, 0x73, 0x74, 0x48, 0x00, 0x52, 0x08, 0x70, 0x61, 0x63, 0x6b, 0x61, 0x67,
	0x65, 0x73, 0x12, 0x16, 0x0a, 0x05, 0x65, 0x72, 0x72, 0x6f, 0x72, 0x18, 0x02, 0x20, 0x01, 0x28,
	0x09, 0x48, 0x00, 0x52, 0x05, 0x65, 0x72, 0x72, 0x6f, 0x72, 0x42, 0x0a, 0x0a, 0x08, 0x72, 0x65,
	0x73, 0x70, 0x6f, 0x6e, 0x73, 0x65, 0x22, 0x53, 0x0a, 0x0f, 0x4c, 0x6f, 0x63, 0x6b, 0x66, 0x69,
	0x6c, 0x65, 0x50, 0x61, 0x63, 0x6b, 0x61, 0x67, 0x65, 0x12, 0x10, 0x0a, 0x03, 0x6b, 0x65, 0x79,
	0x18, 0x01, 0x20, 0x01, 0x28, 0x09, 0x52, 0x03, 0x6b, 0x65, 0x79, 0x12, 0x18, 0x0a, 0x07, 0x76,
	0x65, 0x72, 0x73, 0x69, 0x6f, 0x6e, 0x18, 0x02, 0x20, 0x01, 0x28, 0x09, 0x52, 0x07, 0x76, 0x65,
	0x72, 0x73, 0x69, 0x6f, 0x6e, 0x12, 0x14, 0x0a, 0x05, 0x66, 0x6f, 0x75, 0x6e, 0x64, 0x18, 0x03,
	0x20, 0x01, 0x28, 0x08, 0x52, 0x05, 0x66, 0x6f, 0x75, 0x6e, 0x64, 0x22, 0x3b, 0x0a, 0x13, 0x4c,
	0x6f, 0x63, 0x6b, 0x66, 0x69, 0x6c, 0x65, 0x50, 0x61, 0x63, 0x6b, 0x61, 0x67, 0x65, 0x4c, 0x69,
	0x73, 0x74, 0x12, 0x24, 0x0a, 0x04, 0x6c, 0x69, 0x73, 0x74, 0x18, 0x01, 0x20, 0x03, 0x28, 0x0b,
	0x32, 0x10, 0x2e, 0x4c, 0x6f, 0x63, 0x6b, 0x66, 0x69, 0x6c, 0x65, 0x50, 0x61, 0x63, 0x6b, 0x61,
	0x67, 0x65, 0x52, 0x04, 0x6c, 0x69, 0x73, 0x74, 0x22, 0x69, 0x0a, 0x0f, 0x53, 0x75, 0x62, 0x67,
	0x72, 0x61, 0x70, 0x68, 0x52, 0x65, 0x71, 0x75, 0x65, 0x73, 0x74, 0x12, 0x1a, 0x0a, 0x08, 0x63,
	0x6f, 0x6e, 0x74, 0x65, 0x6e, 0x74, 0x73, 0x18, 0x01, 0x20, 0x01, 0x28, 0x0c, 0x52, 0x08, 0x63,
	0x6f, 0x6e, 0x74, 0x65, 0x6e, 0x74, 0x73, 0x12, 0x1e, 0x0a, 0x0a, 0x77, 0x6f, 0x72, 0x6b, 0x73,
	0x70, 0x61, 0x63, 0x65, 0x73, 0x18, 0x02, 0x20, 0x03, 0x28, 0x09, 0x52, 0x0a, 0x77, 0x6f, 0x72,
	0x6b, 0x73, 0x70, 0x61, 0x63, 0x65, 0x73, 0x12, 0x1a, 0x0a, 0x08, 0x70, 0x61, 0x63, 0x6b, 0x61,
	0x67, 0x65, 0x73, 0x18, 0x03, 0x20, 0x03, 0x28, 0x09, 0x52, 0x08, 0x70, 0x61, 0x63, 0x6b, 0x61,
	0x67, 0x65, 0x73, 0x22, 0x54, 0x0a, 0x10, 0x53, 0x75, 0x62, 0x67, 0x72, 0x61, 0x70, 0x68, 0x52,
	0x65, 0x73, 0x70, 0x6f, 0x6e, 0x73, 0x65, 0x12, 0x1c, 0x0a, 0x08, 0x63, 0x6f, 0x6e, 0x74, 0x65,
	0x6e, 0x74, 0x73, 0x18, 0x01, 0x20, 0x01, 0x28, 0x0c, 0x48, 0x00, 0x52, 0x08, 0x63, 0x6f, 0x6e,
	0x74, 0x65, 0x6e, 0x74, 0x73, 0x12, 0x16, 0x0a, 0x05, 0x65, 0x72, 0x72, 0x6f, 0x72, 0x18, 0x02,
	0x20, 0x01, 0x28, 0x09, 0x48, 0x00, 0x52, 0x05, 0x65, 0x72, 0x72, 0x6f, 0x72, 0x42, 0x0a, 0x0a,
	0x08, 0x72, 0x65, 0x73, 0x70, 0x6f, 0x6e, 0x73, 0x65, 0x42, 0x0b, 0x5a, 0x09, 0x66, 0x66, 0x69,
	0x2f, 0x70, 0x72, 0x6f, 0x74, 0x6f, 0x62, 0x06, 0x70, 0x72, 0x6f, 0x74, 0x6f, 0x33,
}

var (
	file_messages_proto_rawDescOnce sync.Once
	file_messages_proto_rawDescData = file_messages_proto_rawDesc
)

func file_messages_proto_rawDescGZIP() []byte {
	file_messages_proto_rawDescOnce.Do(func() {
		file_messages_proto_rawDescData = protoimpl.X.CompressGZIP(file_messages_proto_rawDescData)
	})
	return file_messages_proto_rawDescData
}

var file_messages_proto_msgTypes = make([]protoimpl.MessageInfo, 11)
var file_messages_proto_goTypes = []interface{}{
	(*TurboDataDirResp)(nil),       // 0: TurboDataDirResp
	(*GlobReq)(nil),                // 1: GlobReq
	(*GlobResp)(nil),               // 2: GlobResp
	(*GlobRespList)(nil),           // 3: GlobRespList
	(*TransitiveDepsRequest)(nil),  // 4: TransitiveDepsRequest
	(*TransitiveDepsResponse)(nil), // 5: TransitiveDepsResponse
	(*LockfilePackage)(nil),        // 6: LockfilePackage
	(*LockfilePackageList)(nil),    // 7: LockfilePackageList
	(*SubgraphRequest)(nil),        // 8: SubgraphRequest
	(*SubgraphResponse)(nil),       // 9: SubgraphResponse
	nil,                            // 10: TransitiveDepsRequest.UnresolvedDepsEntry
}
var file_messages_proto_depIdxs = []int32{
	3,  // 0: GlobResp.files:type_name -> GlobRespList
	10, // 1: TransitiveDepsRequest.unresolved_deps:type_name -> TransitiveDepsRequest.UnresolvedDepsEntry
	7,  // 2: TransitiveDepsResponse.packages:type_name -> LockfilePackageList
	6,  // 3: LockfilePackageList.list:type_name -> LockfilePackage
	4,  // [4:4] is the sub-list for method output_type
	4,  // [4:4] is the sub-list for method input_type
	4,  // [4:4] is the sub-list for extension type_name
	4,  // [4:4] is the sub-list for extension extendee
	0,  // [0:4] is the sub-list for field type_name
}

func init() { file_messages_proto_init() }
func file_messages_proto_init() {
	if File_messages_proto != nil {
		return
	}
	if !protoimpl.UnsafeEnabled {
		file_messages_proto_msgTypes[0].Exporter = func(v interface{}, i int) interface{} {
			switch v := v.(*TurboDataDirResp); i {
			case 0:
				return &v.state
			case 1:
				return &v.sizeCache
			case 2:
				return &v.unknownFields
			default:
				return nil
			}
		}
		file_messages_proto_msgTypes[1].Exporter = func(v interface{}, i int) interface{} {
			switch v := v.(*GlobReq); i {
			case 0:
				return &v.state
			case 1:
				return &v.sizeCache
			case 2:
				return &v.unknownFields
			default:
				return nil
			}
		}
		file_messages_proto_msgTypes[2].Exporter = func(v interface{}, i int) interface{} {
			switch v := v.(*GlobResp); i {
			case 0:
				return &v.state
			case 1:
				return &v.sizeCache
			case 2:
				return &v.unknownFields
			default:
				return nil
			}
		}
		file_messages_proto_msgTypes[3].Exporter = func(v interface{}, i int) interface{} {
			switch v := v.(*GlobRespList); i {
			case 0:
				return &v.state
			case 1:
				return &v.sizeCache
			case 2:
				return &v.unknownFields
			default:
				return nil
			}
		}
		file_messages_proto_msgTypes[4].Exporter = func(v interface{}, i int) interface{} {
			switch v := v.(*TransitiveDepsRequest); i {
			case 0:
				return &v.state
			case 1:
				return &v.sizeCache
			case 2:
				return &v.unknownFields
			default:
				return nil
			}
		}
		file_messages_proto_msgTypes[5].Exporter = func(v interface{}, i int) interface{} {
			switch v := v.(*TransitiveDepsResponse); i {
			case 0:
				return &v.state
			case 1:
				return &v.sizeCache
			case 2:
				return &v.unknownFields
			default:
				return nil
			}
		}
		file_messages_proto_msgTypes[6].Exporter = func(v interface{}, i int) interface{} {
			switch v := v.(*LockfilePackage); i {
			case 0:
				return &v.state
			case 1:
				return &v.sizeCache
			case 2:
				return &v.unknownFields
			default:
				return nil
			}
		}
		file_messages_proto_msgTypes[7].Exporter = func(v interface{}, i int) interface{} {
			switch v := v.(*LockfilePackageList); i {
			case 0:
				return &v.state
			case 1:
				return &v.sizeCache
			case 2:
				return &v.unknownFields
			default:
				return nil
			}
		}
		file_messages_proto_msgTypes[8].Exporter = func(v interface{}, i int) interface{} {
			switch v := v.(*SubgraphRequest); i {
			case 0:
				return &v.state
			case 1:
				return &v.sizeCache
			case 2:
				return &v.unknownFields
			default:
				return nil
			}
		}
		file_messages_proto_msgTypes[9].Exporter = func(v interface{}, i int) interface{} {
			switch v := v.(*SubgraphResponse); i {
			case 0:
				return &v.state
			case 1:
				return &v.sizeCache
			case 2:
				return &v.unknownFields
			default:
				return nil
			}
		}
	}
	file_messages_proto_msgTypes[2].OneofWrappers = []interface{}{
		(*GlobResp_Files)(nil),
		(*GlobResp_Error)(nil),
	}
	file_messages_proto_msgTypes[5].OneofWrappers = []interface{}{
		(*TransitiveDepsResponse_Packages)(nil),
		(*TransitiveDepsResponse_Error)(nil),
	}
	file_messages_proto_msgTypes[9].OneofWrappers = []interface{}{
		(*SubgraphResponse_Contents)(nil),
		(*SubgraphResponse_Error)(nil),
	}
	type x struct{}
	out := protoimpl.TypeBuilder{
		File: protoimpl.DescBuilder{
			GoPackagePath: reflect.TypeOf(x{}).PkgPath(),
			RawDescriptor: file_messages_proto_rawDesc,
			NumEnums:      0,
			NumMessages:   11,
			NumExtensions: 0,
			NumServices:   0,
		},
		GoTypes:           file_messages_proto_goTypes,
		DependencyIndexes: file_messages_proto_depIdxs,
		MessageInfos:      file_messages_proto_msgTypes,
	}.Build()
	File_messages_proto = out.File
	file_messages_proto_rawDesc = nil
	file_messages_proto_goTypes = nil
	file_messages_proto_depIdxs = nil
}

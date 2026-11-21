//go:build darwin

package voxl

// cgo rejects "-Wl,-rpath,@executable_path" in #cgo LDFLAGS (flags starting with "@" are
// not allowed), so only the module's own libs/<triple> directory is on the rpath here.
// A binary that is moved away from the module must add its own rpath when it is linked:
//
//	go build -ldflags="-extldflags '-Wl,-rpath,@executable_path'"
//
// and have libvoxl.dylib next to it. See the README.

/*
#cgo darwin,amd64 LDFLAGS: -L${SRCDIR}/libs/x86_64-apple-darwin -Wl,-rpath,${SRCDIR}/libs/x86_64-apple-darwin -lvoxl
#cgo darwin,arm64 LDFLAGS: -L${SRCDIR}/libs/aarch64-apple-darwin -Wl,-rpath,${SRCDIR}/libs/aarch64-apple-darwin -lvoxl
*/
import "C"

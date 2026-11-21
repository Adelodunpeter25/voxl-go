//go:build darwin

package voxl

/*
#cgo darwin,amd64 LDFLAGS: -L${SRCDIR}/libs/x86_64-apple-darwin -Wl,-rpath,${SRCDIR}/libs/x86_64-apple-darwin -Wl,-rpath,@executable_path -lvoxl
#cgo darwin,arm64 LDFLAGS: -L${SRCDIR}/libs/aarch64-apple-darwin -Wl,-rpath,${SRCDIR}/libs/aarch64-apple-darwin -Wl,-rpath,@executable_path -lvoxl
*/
import "C"

//go:build linux && !musl

package voxl

/*
#cgo linux,amd64 LDFLAGS: -L${SRCDIR}/libs/x86_64-unknown-linux-gnu -Wl,-rpath,${SRCDIR}/libs/x86_64-unknown-linux-gnu -Wl,-rpath,$ORIGIN -lvoxl
#cgo linux,arm64 LDFLAGS: -L${SRCDIR}/libs/aarch64-unknown-linux-gnu -Wl,-rpath,${SRCDIR}/libs/aarch64-unknown-linux-gnu -Wl,-rpath,$ORIGIN -lvoxl
*/
import "C"

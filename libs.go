package voxl

// Shared cgo flags. The per-platform LDFLAGS live in libs_linux_gnu.go,
// libs_linux_musl.go and libs_darwin.go. Every platform links the self-contained
// libvoxl.so/.dylib in libs/<target-triple>/ (x264, opus and libyuv are built into it)
// and records an rpath to it, so `go build` needs no system libraries and no Rust or C
// compilation. To ship a binary, copy the library next to it ($ORIGIN / @executable_path
// are on the rpath too).

/*
#cgo CFLAGS: -I${SRCDIR}/include
*/
import "C"

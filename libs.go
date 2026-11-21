package voxl

// Shared cgo flags. The per-platform LDFLAGS live in libs_linux_gnu.go,
// libs_linux_musl.go and libs_darwin.go. Every platform links the self-contained
// libvoxl.so/.dylib in libs/<target-triple>/ (x264, opus and libyuv are built into it)
// and records an rpath to that directory, so `go build` needs no system libraries and no
// Rust or C compilation.
//
// To ship a binary away from the module directory, put the library next to it. Linux
// finds it through the $ORIGIN rpath set here. macOS can't: cgo refuses
// "-Wl,-rpath,@executable_path" in #cgo LDFLAGS, so the consuming binary has to add that
// rpath itself when it is linked (see the README).

/*
#cgo CFLAGS: -I${SRCDIR}/include
*/
import "C"

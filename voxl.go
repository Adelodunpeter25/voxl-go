// Package voxl streams a desktop as H.264 frames and injects input, through libvoxl.
package voxl

/*
#include <stdlib.h>
#include "voxl.h"

extern void goFrame(uint8_t *data, size_t len, uint32_t kind, void *user);
static void frame_trampoline(const uint8_t *d, size_t n, uint32_t k, void *u) {
	goFrame((uint8_t *)d, n, k, u);
}
static int32_t start(VoxlSession *s, uintptr_t h) {
	return voxl_session_start(s, frame_trampoline, (void *)h);
}
*/
import "C"

import (
	"errors"
	"runtime/cgo"
	"sync"
	"unsafe"
)

const (
	FrameVideo = C.VOXL_FRAME_VIDEO
	FrameText  = C.VOXL_FRAME_TEXT
)

// Config mirrors VoxlConfig; zero values pick defaults.
type Config struct {
	Display      string
	Preset       string
	FPS          uint32
	MaxKbps      uint32
	MinKbps      uint32
	Threads      uint32
	CRF          float32
	EnableResize bool
}

// FrameHandler receives each frame; data is only valid during the call.
type FrameHandler func(kind uint32, data []byte)

type Session struct {
	mu     sync.Mutex
	p      *C.VoxlSession
	handle cgo.Handle
}

func rc(code C.int32_t) error {
	switch code {
	case C.VOXL_OK:
		return nil
	case C.VOXL_ERR_ARG:
		return errors.New("voxl: invalid argument")
	case C.VOXL_ERR_STATE:
		return errors.New("voxl: invalid state")
	case C.VOXL_ERR_INIT:
		return errors.New("voxl: initialisation failed")
	}
	return errors.New("voxl: unknown error")
}

func NewSession(cfg Config) (*Session, error) {
	var c C.VoxlConfig
	if cfg.Display != "" {
		c.display = C.CString(cfg.Display)
		defer C.free(unsafe.Pointer(c.display))
	}
	if cfg.Preset != "" {
		c.preset = C.CString(cfg.Preset)
		defer C.free(unsafe.Pointer(c.preset))
	}
	c.fps, c.max_kbps, c.min_kbps, c.threads = C.uint32_t(cfg.FPS), C.uint32_t(cfg.MaxKbps), C.uint32_t(cfg.MinKbps), C.uint32_t(cfg.Threads)
	c.crf = C.float(cfg.CRF)
	if cfg.EnableResize {
		c.enable_resize = 1
	}
	p := C.voxl_session_create(&c)
	if p == nil {
		return nil, errors.New("voxl: create failed")
	}
	return &Session{p: p}, nil
}

//export goFrame
func goFrame(data *C.uint8_t, n C.size_t, kind C.uint32_t, user unsafe.Pointer) {
	h := cgo.Handle(uintptr(user))
	fn := h.Value().(FrameHandler)
	fn(uint32(kind), unsafe.Slice((*byte)(unsafe.Pointer(data)), int(n)))
}

func (s *Session) Start(fn FrameHandler) error {
	s.mu.Lock()
	defer s.mu.Unlock()
	if s.p == nil {
		return errors.New("voxl: session destroyed")
	}
	s.handle = cgo.NewHandle(fn)
	if err := rc(C.start(s.p, C.uintptr_t(s.handle))); err != nil {
		s.handle.Delete()
		s.handle = 0
		return err
	}
	return nil
}

func (s *Session) Stop() error {
	s.mu.Lock()
	defer s.mu.Unlock()
	if s.p == nil {
		return nil
	}
	return rc(C.voxl_session_stop(s.p))
}

func (s *Session) SendMouse(x, y int32, buttonMask uint32, wheel int32) error {
	return rc(C.voxl_session_send_mouse(s.p, C.int32_t(x), C.int32_t(y), C.uint32_t(buttonMask), C.int32_t(wheel)))
}

func (s *Session) SendKey(keysym uint32, down bool) error {
	d := C.int32_t(0)
	if down {
		d = 1
	}
	return rc(C.voxl_session_send_key(s.p, C.uint32_t(keysym), d))
}

func (s *Session) Resize(w, h uint32) error {
	return rc(C.voxl_session_resize(s.p, C.uint32_t(w), C.uint32_t(h)))
}

// Close stops and frees the session.
func (s *Session) Close() {
	s.mu.Lock()
	defer s.mu.Unlock()
	if s.p == nil {
		return
	}
	C.voxl_session_destroy(s.p)
	s.p = nil
	if s.handle != 0 {
		s.handle.Delete()
	}
}

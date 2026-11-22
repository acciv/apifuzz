package generator

/*
#cgo CFLAGS: -I${SRCDIR}/../../../rust/include
#cgo LDFLAGS: -L${SRCDIR}/../../../rust/target/release -lapifuzz_core -Wl,-rpath,${SRCDIR}/../../../rust/target/release
#include <stdlib.h>
#include "apifuzz_core.h"
*/
import "C"

import (
	"encoding/json"
	"errors"
	"fmt"
	"unsafe"
)

type Case struct {
	Name           string `json:"name"`
	Method         string `json:"method"`
	Path           string `json:"path"`
	Query          string `json:"query"`
	Body           string `json:"body"`
	Kind           string `json:"kind"`
	ExpectedStatus []int  `json:"expected_status"`
}

type Report struct {
	Version    string `json:"version"`
	Operations int    `json:"operations"`
	Cases      []Case `json:"cases"`
}

func Version() string {
	return C.GoString(C.apifuzz_version())
}

// Generate прогоняет OpenAPI-спецификацию через Rust-ядро.
func Generate(spec []byte) (*Report, error) {
	ptr := C.CString(string(spec))
	defer C.free(unsafe.Pointer(ptr))

	out := C.apifuzz_generate(ptr)
	if out == nil {
		return nil, errors.New("generator returned null")
	}
	defer C.apifuzz_free(out)

	var report Report
	var raw struct {
		Version    string `json:"version"`
		Operations int    `json:"operations"`
		Cases      []Case `json:"cases"`
		Error      string `json:"error"`
	}
	if err := json.Unmarshal([]byte(C.GoString(out)), &raw); err != nil {
		return nil, fmt.Errorf("decode generator output: %w", err)
	}
	if raw.Error != "" {
		return nil, errors.New(raw.Error)
	}
	if len(raw.Cases) == 0 {
		return nil, errors.New("no cases generated")
	}
	report.Version = raw.Version
	report.Operations = raw.Operations
	report.Cases = raw.Cases
	return &report, nil
}

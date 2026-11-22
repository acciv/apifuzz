package runner

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"testing"
	"time"

	"github.com/acciv/apifuzz/internal/generator"
)

func TestRunAndWrite(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		_, _ = w.Write([]byte(`{"ok":true}`))
	}))
	defer server.Close()

	cases := []generator.Case{
		{Name: "one", Method: "GET", Path: "/a", ExpectedStatus: []int{200}},
		{Name: "two", Method: "GET", Path: "/b", Query: "x=1", ExpectedStatus: []int{200}},
		{Name: "three", Method: "POST", Path: "/c", Body: `{"v":1}`, ExpectedStatus: []int{200}},
		{Name: "four", Method: "GET", Path: "/d", ExpectedStatus: []int{200}},
	}

	results := New(server.URL, 2, 2*time.Second).Run(cases)
	if len(results) != len(cases) {
		t.Fatalf("got %d results", len(results))
	}
	for i, result := range results {
		if result.Index != i || result.Status != 200 || result.Error != "" {
			t.Fatalf("bad result %d: %+v", i, result)
		}
	}

	path := filepath.Join(t.TempDir(), "results.jsonl")
	if err := WriteJSONL(path, results); err != nil {
		t.Fatal(err)
	}
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	if len(data) == 0 {
		t.Fatal("empty results file")
	}
	var first Result
	if err := json.Unmarshal(bytesFirstLine(data), &first); err != nil {
		t.Fatal(err)
	}
	if first.Status != 200 {
		t.Fatalf("bad saved status: %d", first.Status)
	}
}

func bytesFirstLine(data []byte) []byte {
	for i, b := range data {
		if b == '\n' {
			return data[:i]
		}
	}
	return data
}

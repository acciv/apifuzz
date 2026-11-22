package runner

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"os"
	"sort"
	"strings"
	"sync"
	"time"

	"github.com/acciv/apifuzz/internal/generator"
)

type Result struct {
	Index        int    `json:"index"`
	Name         string `json:"name"`
	Method       string `json:"method"`
	Path         string `json:"path"`
	Query        string `json:"query"`
	Kind         string `json:"kind"`
	Expected     []int  `json:"expected_status"`
	Status       int    `json:"status"`
	ResponseBody string `json:"response_body"`
	DurationMS   float64 `json:"duration_ms"`
	Error        string `json:"error,omitempty"`
}

type Runner struct {
	baseURL string
	client  *http.Client
	workers int
}

func New(baseURL string, workers int, timeout time.Duration) *Runner {
	if workers < 1 {
		workers = 1
	}
	return &Runner{
		baseURL: strings.TrimRight(baseURL, "/"),
		client:  &http.Client{Timeout: timeout},
		workers: workers,
	}
}

func (r *Runner) Run(cases []generator.Case) []Result {
	jobs := make(chan int)
	results := make(chan Result, len(cases))
	var wg sync.WaitGroup

	for i := 0; i < r.workers; i++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			for index := range jobs {
				results <- r.execute(index, cases[index])
			}
		}()
	}
	for i := range cases {
		jobs <- i
	}
	close(jobs)
	wg.Wait()
	close(results)

	out := make([]Result, 0, len(cases))
	for result := range results {
		out = append(out, result)
	}
	sort.Slice(out, func(i, j int) bool { return out[i].Index < out[j].Index })
	return out
}

func (r *Runner) execute(index int, testCase generator.Case) Result {
	result := Result{
		Index:    index,
		Name:     testCase.Name,
		Method:   testCase.Method,
		Path:     testCase.Path,
		Query:    testCase.Query,
		Kind:     testCase.Kind,
		Expected: testCase.ExpectedStatus,
	}
	target := r.baseURL + testCase.Path
	if testCase.Query != "" {
		target += "?" + testCase.Query
	}

	var body io.Reader
	if testCase.Body != "" {
		body = bytes.NewBufferString(testCase.Body)
	}
	req, err := http.NewRequest(testCase.Method, target, body)
	if err != nil {
		result.Error = err.Error()
		return result
	}
	req.Header.Set("User-Agent", "apifuzz/"+generator.Version())
	if testCase.Body != "" {
		req.Header.Set("Content-Type", "application/json")
	}

	start := time.Now()
	resp, err := r.client.Do(req)
	result.DurationMS = float64(time.Since(start).Microseconds()) / 1000
	if err != nil {
		result.Error = err.Error()
		return result
	}
	defer resp.Body.Close()

	data, err := io.ReadAll(io.LimitReader(resp.Body, 64<<10))
	if err != nil {
		result.Error = err.Error()
	}
	result.Status = resp.StatusCode
	result.ResponseBody = string(data)
	if len(result.ResponseBody) > 4096 {
		result.ResponseBody = result.ResponseBody[:4096]
	}
	return result
}

func WriteJSONL(path string, results []Result) error {
	file, err := os.Create(path)
	if err != nil {
		return err
	}
	defer file.Close()

	encoder := json.NewEncoder(file)
	for _, result := range results {
		if err := encoder.Encode(result); err != nil {
			return fmt.Errorf("encode result: %w", err)
		}
	}
	return nil
}

package main

import (
	"flag"
	"fmt"
	"log"
	"os"
	"time"

	"github.com/acciv/apifuzz/internal/generator"
	"github.com/acciv/apifuzz/internal/runner"
)

func main() {
	specPath := flag.String("spec", "examples/openapi.json", "OpenAPI JSON file")
	baseURL := flag.String("base-url", "http://127.0.0.1:9101", "target API base URL")
	outPath := flag.String("out", "results.jsonl", "results JSONL file")
	workers := flag.Int("workers", 4, "parallel workers")
	timeoutMS := flag.Int("timeout-ms", 3000, "request timeout in milliseconds")
	showVersion := flag.Bool("version", false, "print versions and exit")
	flag.Parse()

	if *showVersion {
		fmt.Printf("apifuzz core %s\n", generator.Version())
		return
	}

	spec, err := os.ReadFile(*specPath)
	if err != nil {
		log.Fatalf("spec: %v", err)
	}
	report, err := generator.Generate(spec)
	if err != nil {
		log.Fatalf("generate: %v", err)
	}
	log.Printf("core %s: %d operations -> %d cases", report.Version, report.Operations, len(report.Cases))

	run := runner.New(*baseURL, *workers, time.Duration(*timeoutMS)*time.Millisecond)
	results := run.Run(report.Cases)
	if err := runner.WriteJSONL(*outPath, results); err != nil {
		log.Fatalf("write results: %v", err)
	}
	log.Printf("done: %d results -> %s", len(results), *outPath)
}

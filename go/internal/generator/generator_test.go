package generator

import "testing"

const spec = `{
  "openapi":"3.0.3",
  "info":{"title":"demo","version":"1.0.0"},
  "paths":{
    "/users":{"post":{"requestBody":{"content":{"application/json":{"schema":{
      "type":"object","properties":{"name":{"type":"string"}},"required":["name"]
    }}}},"responses":{"201":{},"400":{}}}}
  }
}`

func TestGenerate(t *testing.T) {
	report, err := Generate([]byte(spec))
	if err != nil {
		t.Fatal(err)
	}
	if report.Operations != 1 || len(report.Cases) < 4 {
		t.Fatalf("unexpected report: %+v", report)
	}
	found := false
	for _, item := range report.Cases {
		if item.Method == "POST" && item.Path == "/users" && item.Kind == "valid" {
			found = true
		}
	}
	if !found {
		t.Fatal("baseline case missing")
	}
}

func TestInvalidSpec(t *testing.T) {
	if _, err := Generate([]byte("nope")); err == nil {
		t.Fatal("invalid spec accepted")
	}
}

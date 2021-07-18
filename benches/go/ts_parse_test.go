package main

import (
	"regexp"
	"testing"
)

func BenchmarkRegexRFC3339(b *testing.B) {
	rfc3339 := []byte("2020-06-27T00:48:30.466249792+03:00")
	expr := regexp.MustCompile(`^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?(Z|[+-](\d{2}:\d{2}))?$`)
	for i := 0; i < b.N; i++ {
		if !expr.Match(rfc3339) {
			panic("not matched")
		}
	}
}

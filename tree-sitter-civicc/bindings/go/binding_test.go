package tree_sitter_civicc_test

import (
	"testing"

	tree_sitter "github.com/tree-sitter/go-tree-sitter"
	tree_sitter_civicc "http://example.com//bindings/go"
)

func TestCanLoadGrammar(t *testing.T) {
	language := tree_sitter.NewLanguage(tree_sitter_civicc.Language())
	if language == nil {
		t.Errorf("Error loading civicc grammar")
	}
}

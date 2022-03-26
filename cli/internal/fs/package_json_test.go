package fs

import (
	"os"
	"path/filepath"
	"testing"

	"github.com/stretchr/testify/assert"
)

func Test_ParseTurboConfigJson(t *testing.T) {
	defaultCwd, err := os.Getwd()
	if err != nil {
		t.Errorf("failed to get cwd: %v", err)
	}
	turboJSONPath := filepath.Join(defaultCwd, "testdata", "turbo.json")
	turboConfig, err := ReadTurboConfigJSON(turboJSONPath)
	if err != nil {
		t.Fatalf("invalid parse: %#v", err)
	}
	BoolFalse := false

	build := Pipeline{[]string{"dist/**", ".next/**"}, nil, []string{"^build"}, PPipeline{&[]string{"dist/**", ".next/**"}, nil, []string{"^build"}}}
	lint := Pipeline{[]string{}, nil, nil, PPipeline{&[]string{}, nil, nil}}
	dev := Pipeline{nil, &BoolFalse, nil, PPipeline{nil, &BoolFalse, nil}}
	pipelineExpected := map[string]Pipeline{"build": build, "lint": lint, "dev": dev}

	remoteCacheOptionsExpected := RemoteCacheOptions{"team_id", SignatureOptions{true, "key", ""}}
	assert.EqualValues(t, pipelineExpected, turboConfig.Pipeline)
	assert.EqualValues(t, remoteCacheOptionsExpected, turboConfig.RemoteCacheOptions)
}

package util

import (
	"fmt"
	"io/ioutil"
	"path/filepath"

	"github.com/Masterminds/semver"
	"gopkg.in/yaml.v3"
)

type YarnRC struct {
	NodeLinker string `yaml:"nodeLinker"`
}

func IsYarn(backendName string) bool {
	return backendName == "nodejs-yarn" || backendName == "nodejs-berry"
}

func IsNMLinker(cwd string) (bool, error) {
	yarnRC := &YarnRC{}

	bytes, err := ioutil.ReadFile(filepath.Join(cwd, ".yarnrc.yml"))
	if err != nil {
		return false, fmt.Errorf(".yarnrc.yml: %w", err)
	}

	if yaml.Unmarshal(bytes, yarnRC) != nil {
		return false, fmt.Errorf(".yarnrc.yml: %w", err)
	}

	return yarnRC.NodeLinker == "node-modules", nil
}

// MustCompileSemverConstraint compiles the given text into a constraint
// and panics on error. Intended for uses where an error indicates a programming
// error and we should crash ASAP.
func MustCompileSemverConstraint(text string) *semver.Constraints {
	c, err := semver.NewConstraint(text)
	if err != nil {
		panic(err)
	}
	return c
}

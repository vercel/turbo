package env

import (
	"crypto/sha256"
	"fmt"
	"os"
	"regexp"
	"sort"
	"strings"
)

// EnvironmentVariableMap is a map of env variables and their values
type EnvironmentVariableMap map[string]string

// BySource contains a map of environment variables broken down by the source
type BySource struct {
	Explicit EnvironmentVariableMap
	Matching EnvironmentVariableMap
}

// DetailedMap contains the composite and the detailed maps of environment variables
// All is used as a taskhash input (taskhash.CalculateTaskHash)
// BySoure is used to print out a Dry Run Summary
type DetailedMap struct {
	All      EnvironmentVariableMap
	BySource BySource
}

// Merge takes another EnvironmentVariableMap and merges it into the receiver
// It overwrites values if they already exist, but since the source of both will be os.Environ()
// it doesn't matter
func (evm EnvironmentVariableMap) Merge(another EnvironmentVariableMap) {
	for k, v := range another {
		evm[k] = v
	}
}

// EnvironmentVariablePairs is a list of "k=v" strings for env variables and their values
type EnvironmentVariablePairs []string

// mapToPair returns a deterministically sorted set of EnvironmentVariablePairs from an EnvironmentVariableMap
// It takes a transformer value to operate on each key-value pair and return a string
func (evm EnvironmentVariableMap) mapToPair(transformer func(k string, v string) string) EnvironmentVariablePairs {
	// convert to set to eliminate duplicates
	pairs := make([]string, 0, len(evm))
	for k, v := range evm {
		paired := transformer(k, v)
		pairs = append(pairs, paired)
	}

	// sort it so it's deterministic
	sort.Strings(pairs)

	return pairs
}

// ToSecretHashable returns a deterministically sorted set of EnvironmentVariablePairs from an EnvironmentVariableMap
// This is the value used to print out the task hash input, so the values are cryptographically hashed
func (evm EnvironmentVariableMap) ToSecretHashable() EnvironmentVariablePairs {
	return evm.mapToPair(func(k, v string) string {
		hashedValue := sha256.Sum256([]byte(v))
		return fmt.Sprintf("%v=%x", k, hashedValue)
	})
}

// ToHashable returns a deterministically sorted set of EnvironmentVariablePairs from an EnvironmentVariableMap
// This is the value that is used upstream as a task hash input, so we need it to be deterministic
func (evm EnvironmentVariableMap) ToHashable() EnvironmentVariablePairs {
	return evm.mapToPair(func(k, v string) string {
		return fmt.Sprintf("%v=%v", k, v)
	})
}

func getEnvMap() EnvironmentVariableMap {
	envMap := make(map[string]string)
	for _, envVar := range os.Environ() {
		if i := strings.Index(envVar, "="); i >= 0 {
			parts := strings.SplitN(envVar, "=", 2)
			envMap[parts[0]] = strings.Join(parts[1:], "")
		}
	}
	return envMap
}

// fromKeys returns a map of env vars and their values based on include/exclude prefixes
func fromKeys(all EnvironmentVariableMap, keys []string) EnvironmentVariableMap {
	output := EnvironmentVariableMap{}
	for _, key := range keys {
		output[key] = all[key]
	}

	return output
}

func fromMatching(all EnvironmentVariableMap, keyMatchers []string, excludePrefix string) (EnvironmentVariableMap, error) {
	output := EnvironmentVariableMap{}
	compileFailures := []string{}

	for _, keyMatcher := range keyMatchers {
		rex, err := regexp.Compile(keyMatcher)
		if err != nil {
			compileFailures = append(compileFailures, keyMatcher)
			continue
		}

		for k, v := range all {
			// we can skip the keys that match the excludedPrefix
			if excludePrefix != "" && strings.HasPrefix(k, excludePrefix) {
				continue
			}

			if rex.Match([]byte(k)) {
				output[k] = v
			}
		}
	}

	if len(compileFailures) > 0 {
		return nil, fmt.Errorf("The following env prefixes failed to compile to regex: %s", strings.Join(compileFailures, ", "))
	}

	return output, nil
}

// GetHashableEnvVars returns all sorted key=value env var pairs for both frameworks and from envKeys
func GetHashableEnvVars(keys []string, matchers []string, excludePrefix string) (DetailedMap, error) {
	all := getEnvMap()

	detailedMap := DetailedMap{
		All:      EnvironmentVariableMap{},
		BySource: BySource{},
	}

	detailedMap.BySource.Explicit = fromKeys(all, keys)
	detailedMap.All.Merge(detailedMap.BySource.Explicit)

	matchedEnvVars, err := fromMatching(all, matchers, all[excludePrefix])

	if err != nil {
		return DetailedMap{}, err
	}

	detailedMap.BySource.Matching = matchedEnvVars
	detailedMap.All.Merge(detailedMap.BySource.Matching)
	return detailedMap, nil
}

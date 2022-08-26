package lockfile

import (
	"bufio"
	"bytes"
	"fmt"
	"io"
	"regexp"
	"strings"

	"github.com/pkg/errors"
	"gopkg.in/yaml.v3"
)

var rnLineEnding = regexp.MustCompile("\"|:\r\n$")
var nLineEnding = regexp.MustCompile("\"|:\n$")
<<<<<<< HEAD
var lineStart = regexp.MustCompile(`^[\w"]`)
var double = regexp.MustCompile(`\:\"\:`)
var quotedWhitespace = regexp.MustCompile(`\"\s\"`)
=======
var r = regexp.MustCompile(`^[\w"]`)
var double = regexp.MustCompile(`\:\"\:`)
var o = regexp.MustCompile(`\"\s\"`)
>>>>>>> df744a10 (Move lockfile reading operation into package manager abstraction)

// deals with colons
// integrity sha-... -> integrity: sha-...
// "@apollo/client" latest -> "@apollo/client": latest
// "@apollo/client" "0.0.0" -> "@apollo/client": "0.0.0"
// apollo-client "0.0.0" -> apollo-client: "0.0.0"
<<<<<<< HEAD
var spaceDelimitedChars = regexp.MustCompile(`(\w|\")\s(\"|\w)`)

// YarnLockfileEntry package information from yarn lockfile
=======
var a = regexp.MustCompile(`(\w|\")\s(\"|\w)`)

>>>>>>> df744a10 (Move lockfile reading operation into package manager abstraction)
type YarnLockfileEntry struct {
	// resolved version for the particular entry based on the provided semver revision
	Version   string `yaml:"version"`
	Resolved  string `yaml:"resolved"`
	Integrity string `yaml:"integrity"`
	// the list of unresolved modules and revisions (e.g. type-detect : ^4.0.0)
	Dependencies map[string]string `yaml:"dependencies,omitempty"`
	// the list of unresolved modules and revisions (e.g. type-detect : ^4.0.0)
	OptionalDependencies map[string]string `yaml:"optionalDependencies,omitempty"`
}

<<<<<<< HEAD
// YarnLockfile representation of yarn lockfile
=======
>>>>>>> df744a10 (Move lockfile reading operation into package manager abstraction)
type YarnLockfile map[string]*YarnLockfileEntry

var _ Lockfile = (*YarnLockfile)(nil)

<<<<<<< HEAD
// ResolvePackage Given a package and version returns the key, resolved version, and if it was found
func (l *YarnLockfile) ResolvePackage(name string, version string) (string, string, bool) {
	for _, key := range yarnPossibleKeys(name, version) {
=======
func (l *YarnLockfile) PossibleKeys(name string, version string) []string {
	return yarnPossibleKeys(name, version)
}

func (l *YarnLockfile) ResolvePackage(name string, version string) (string, string, bool) {
	for _, key := range l.PossibleKeys(name, version) {
>>>>>>> df744a10 (Move lockfile reading operation into package manager abstraction)
		if entry, ok := (*l)[key]; ok {
			return key, entry.Version, true
		}
	}

	return "", "", false
}

<<<<<<< HEAD
// AllDependencies Given a lockfile key return all (dev/optional/peer) dependencies of that package
=======
>>>>>>> df744a10 (Move lockfile reading operation into package manager abstraction)
func (l *YarnLockfile) AllDependencies(key string) (map[string]string, bool) {
	deps := map[string]string{}
	entry, ok := (*l)[key]
	if !ok {
		return deps, false
	}

	for name, version := range entry.Dependencies {
		deps[name] = version
	}
	for name, version := range entry.OptionalDependencies {
		deps[name] = version
	}

	return deps, true
}

<<<<<<< HEAD
// Subgraph Given a list of lockfile keys returns a Lockfile based off the original one that only contains the packages given
func (l *YarnLockfile) Subgraph(packages []string) (Lockfile, error) {
=======
func (l *YarnLockfile) SubLockfile(packages []string) (Lockfile, error) {
>>>>>>> df744a10 (Move lockfile reading operation into package manager abstraction)
	lockfile := make(YarnLockfile, len(packages))
	for _, key := range packages {
		entry, ok := (*l)[key]
		if ok {
			lockfile[key] = entry
		}
	}

	return &lockfile, nil
}

<<<<<<< HEAD
// Encode encode the lockfile representation and write it to the given writer
=======
>>>>>>> df744a10 (Move lockfile reading operation into package manager abstraction)
func (l *YarnLockfile) Encode(w io.Writer) error {
	var tmp bytes.Buffer
	yamlEncoder := yaml.NewEncoder(&tmp)
	yamlEncoder.SetIndent(2)
	if err := yamlEncoder.Encode(l); err != nil {
		return errors.Wrap(err, "failed to materialize sub-lockfile. This can happen if your lockfile contains merge conflicts or is somehow corrupted. Please report this if it occurs")
	}

<<<<<<< HEAD
	if _, err := io.WriteString(w, "# THIS IS AN AUTOGENERATED FILE. DO NOT EDIT THIS FILE DIRECTLY.\n# yarn lockfile v1\n\n"); err != nil {
		return errors.Wrap(err, "failed to write to buffer")
	}
=======
	io.WriteString(w, "# THIS IS AN AUTOGENERATED FILE. DO NOT EDIT THIS FILE DIRECTLY.\n# yarn lockfile v1\n\n")
>>>>>>> df744a10 (Move lockfile reading operation into package manager abstraction)

	// because of yarn being yarn, we need to inject lines in between each block of YAML to make it "valid" SYML
	scan := bufio.NewScanner(&tmp)
	buf := make([]byte, 0, 1024*1024)
	scan.Buffer(buf, 10*1024*1024)
	for scan.Scan() {
		line := scan.Text() //Writing to Stdout
<<<<<<< HEAD
		var stringToWrite string
		if !strings.HasPrefix(line, " ") {
			stringToWrite = fmt.Sprintf("\n%v\n", strings.ReplaceAll(line, "'", "\""))
		} else {
			stringToWrite = fmt.Sprintf("%v\n", strings.ReplaceAll(line, "'", "\""))
		}
		if _, err := io.WriteString(w, stringToWrite); err != nil {
			return errors.Wrap(err, "failed to write to buffer")
=======
		if !strings.HasPrefix(line, " ") {
			io.WriteString(w, fmt.Sprintf("\n%v\n", strings.ReplaceAll(line, "'", "\"")))
		} else {
			io.WriteString(w, fmt.Sprintf("%v\n", strings.ReplaceAll(line, "'", "\"")))
>>>>>>> df744a10 (Move lockfile reading operation into package manager abstraction)
		}
	}

	return nil
}

// DecodeYarnLockfile Takes the contents of a yarn lockfile and returns a struct representation
func DecodeYarnLockfile(contents []byte) (*YarnLockfile, error) {
	var lockfile map[string]*YarnLockfileEntry

	var next []byte
	var lines []string
	var l *regexp.Regexp
	var output string

	hasLF := !bytes.HasSuffix(contents, []byte("\r\n"))
	if hasLF {
		lines = strings.Split(string(contents), "\n")
		l = nLineEnding
	} else {
		lines = strings.Split(strings.TrimRight(string(contents), "\r\n"), "\r\n")
		l = rnLineEnding
	}

	for i, line := range lines {
<<<<<<< HEAD
		if lineStart.MatchString(line) {
=======
		if r.MatchString(line) {
>>>>>>> df744a10 (Move lockfile reading operation into package manager abstraction)
			first := fmt.Sprintf("\"%v\":", l.ReplaceAllString(line, ""))
			lines[i] = double.ReplaceAllString(first, "\":")
		}
	}

	if hasLF {
<<<<<<< HEAD
		output = quotedWhitespace.ReplaceAllString(strings.Join(lines, "\n"), "\": \"")
	} else {
		output = quotedWhitespace.ReplaceAllString(strings.Join(lines, "\r\n"), "\": \"")
	}

	next = []byte(spaceDelimitedChars.ReplaceAllStringFunc(output, func(m string) string {
		parts := spaceDelimitedChars.FindStringSubmatch(m)
=======
		output = o.ReplaceAllString(strings.Join(lines, "\n"), "\": \"")
	} else {
		output = o.ReplaceAllString(strings.Join(lines, "\r\n"), "\": \"")
	}

	next = []byte(a.ReplaceAllStringFunc(output, func(m string) string {
		parts := a.FindStringSubmatch(m)
>>>>>>> df744a10 (Move lockfile reading operation into package manager abstraction)
		return fmt.Sprintf("%s: %s", parts[1], parts[2])
	}))

	err := yaml.Unmarshal(next, &lockfile)
	if err != nil {
		return nil, fmt.Errorf("could not unmarshal lockfile: %w", err)
	}

	prettyLockfile := YarnLockfile(yarnSplitOutEntries(lockfile))
	return &prettyLockfile, nil
}

func yarnPossibleKeys(name string, version string) []string {
	return []string{
		fmt.Sprintf("%v@%v", name, version),
		fmt.Sprintf("%v@npm:%v", name, version),
<<<<<<< HEAD
		fmt.Sprintf("%v@file:%v", name, version),
		fmt.Sprintf("%v@workspace:%v", name, version),
		fmt.Sprintf("%v@yarn:%v", name, version),
=======
>>>>>>> df744a10 (Move lockfile reading operation into package manager abstraction)
	}
}

func yarnSplitOutEntries(lockfile map[string]*YarnLockfileEntry) map[string]*YarnLockfileEntry {
	prettyLockfile := map[string]*YarnLockfileEntry{}
	// This final step is important, it splits any deps with multiple-resolutions
	// (e.g. "@babel/generator@^7.13.0, @babel/generator@^7.13.9":) into separate
	// entries in our map
	// TODO: make concurrent
	for key, val := range lockfile {
		if strings.Contains(key, ",") {
			for _, v := range strings.Split(key, ", ") {
				prettyLockfile[strings.TrimSpace(v)] = val
			}
		} else {
			prettyLockfile[key] = val
		}
	}
	return prettyLockfile
}

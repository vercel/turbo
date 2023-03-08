package runsummary

import (
	"fmt"
	"os"
	"sync"
	"time"

	"github.com/vercel/turbo/cli/internal/chrometracing"
	"github.com/vercel/turbo/cli/internal/fs"
	"github.com/vercel/turbo/cli/internal/ui"
	"github.com/vercel/turbo/cli/internal/util"

	"github.com/fatih/color"
	"github.com/mitchellh/cli"
)

// RunResult represents a single event in the build process, i.e. a target starting or finishing
// building, or reaching some milestone within those steps.
type RunResult struct {
	// Timestamp of this event
	Time time.Time
	// Duration of this event
	Duration time.Duration
	// Target which has just changed
	Label string
	// Its current status
	Status RunResultStatus
	// Error, only populated for failure statuses
	Err error
}

// RunResultStatus represents the status of a target when we log a build result.
type RunResultStatus int

// The collection of expected build result statuses.
const (
	TargetBuilding RunResultStatus = iota
	TargetBuildStopped
	TargetBuilt
	TargetCached
	TargetBuildFailed
)

func (rrs RunResultStatus) toString() string {
	switch rrs {
	case TargetBuilding:
		return "building"
	case TargetBuildStopped:
		return "buildStopped"
	case TargetBuilt:
		return "built"
	case TargetCached:
		return "cached"
	case TargetBuildFailed:
		return "buildFailed"
	}

	return ""
}

// BuildTargetState contains data about the state of a single task in a turbo run.
// Some fields are updated over time as the task prepares to execute and finishes execution.
type BuildTargetState struct {
	StartAt time.Time `json:"start"`

	Duration time.Duration `json:"duration"`

	// Target which has just changed
	Label string `json:"-"`

	// Its current status
	Status string `json:"status"`

	// Error, only populated for failure statuses
	Err error `json:"error"`
}

// RunState is the state of the entire `turbo run`. Individual task state in `Tasks` field
// TODO(mehulkar): Can this be combined with the RunSummary?
type RunState struct {
	mu      sync.Mutex
	state   map[string]*BuildTargetState
	Success int
	Failure int
	// Is the output streaming?
	Cached    int
	Attempted int

	startedAt time.Time

	profileFilename string
}

// NewRunState creates a RunState instance to track events in a `turbo run`.`
func NewRunState(start time.Time, tracingProfile string) *RunState {
	if tracingProfile != "" {
		chrometracing.EnableTracing()
	}

	return &RunState{
		Success:         0,
		Failure:         0,
		Cached:          0,
		Attempted:       0,
		state:           make(map[string]*BuildTargetState),
		startedAt:       start,
		profileFilename: tracingProfile,
	}
}

// Run starts the Execution of a single task. It returns a function that can
// be used to update the state of a given taskID with the RunResultStatus enum
func (r *RunState) Run(label string) (func(outcome RunResultStatus, err error), *BuildTargetState) {
	start := time.Now()
	buildTargetState := r.add(&RunResult{
		Time:   start,
		Label:  label,
		Status: TargetBuilding,
	}, label, true)

	tracer := chrometracing.Event(label)

	// This function can be called with an enum and an optional error to update
	// the state of a given taskID.
	tracerFn := func(outcome RunResultStatus, err error) {
		defer tracer.Done()
		now := time.Now()
		result := &RunResult{
			Time:     now,
			Duration: now.Sub(start),
			Label:    label,
			Status:   outcome,
		}
		if err != nil {
			result.Err = fmt.Errorf("running %v failed: %w", label, err)
		}
		// Ignore the return value here
		r.add(result, label, false)
	}

	return tracerFn, buildTargetState
}

func (r *RunState) add(result *RunResult, previous string, active bool) *BuildTargetState {
	r.mu.Lock()
	defer r.mu.Unlock()
	if s, ok := r.state[result.Label]; ok {
		s.Status = result.Status.toString()
		s.Err = result.Err
		s.Duration = result.Duration
	} else {
		r.state[result.Label] = &BuildTargetState{
			StartAt:  result.Time,
			Label:    result.Label,
			Status:   result.Status.toString(),
			Err:      result.Err,
			Duration: result.Duration,
		}
	}
	switch {
	case result.Status == TargetBuildFailed:
		r.Failure++
		r.Attempted++
	case result.Status == TargetCached:
		r.Cached++
		r.Attempted++
	case result.Status == TargetBuilt:
		r.Success++
		r.Attempted++
	}

	return r.state[result.Label]
}

// Close finishes a trace of a turbo run. The tracing file will be written if applicable,
// and run stats are written to the terminal
func (r *RunState) Close(terminal cli.Ui) error {
	if err := writeChrometracing(r.profileFilename, terminal); err != nil {
		terminal.Error(fmt.Sprintf("Error writing tracing data: %v", err))
	}

	maybeFullTurbo := ""
	if r.Cached == r.Attempted && r.Attempted > 0 {
		terminalProgram := os.Getenv("TERM_PROGRAM")
		// On the macOS Terminal, the rainbow colors show up as a magenta background
		// with a gray background on a single letter. Instead, we print in bold magenta
		if terminalProgram == "Apple_Terminal" {
			fallbackTurboColor := color.New(color.FgHiMagenta, color.Bold).SprintFunc()
			maybeFullTurbo = fallbackTurboColor(">>> FULL TURBO")
		} else {
			maybeFullTurbo = ui.Rainbow(">>> FULL TURBO")
		}
	}

	if r.Attempted == 0 {
		terminal.Output("") // Clear the line
		terminal.Warn("No tasks were executed as part of this run.")
	}
	terminal.Output("") // Clear the line
	terminal.Output(util.Sprintf("${BOLD} Tasks:${BOLD_GREEN}    %v successful${RESET}${GRAY}, %v total${RESET}", r.Cached+r.Success, r.Attempted))
	terminal.Output(util.Sprintf("${BOLD}Cached:    %v cached${RESET}${GRAY}, %v total${RESET}", r.Cached, r.Attempted))
	terminal.Output(util.Sprintf("${BOLD}  Time:    %v${RESET} %v${RESET}", time.Since(r.startedAt).Truncate(time.Millisecond), maybeFullTurbo))
	terminal.Output("")
	return nil
}

// writeChromeTracing writes to a profile name if the `--profile` flag was passed to turbo run
func writeChrometracing(filename string, terminal cli.Ui) error {
	outputPath := chrometracing.Path()
	if outputPath == "" {
		// tracing wasn't enabled
		return nil
	}

	name := fmt.Sprintf("turbo-%s.trace", time.Now().Format(time.RFC3339))
	if filename != "" {
		name = filename
	}
	if err := chrometracing.Close(); err != nil {
		terminal.Warn(fmt.Sprintf("Failed to flush tracing data: %v", err))
	}
	cwdRaw, err := os.Getwd()
	if err != nil {
		return err
	}
	root, err := fs.GetCwd(cwdRaw)
	if err != nil {
		return err
	}
	// chrometracing.Path() is absolute by default, but can still be relative if overriden via $CHROMETRACING_DIR
	// so we have to account for that before converting to turbopath.AbsoluteSystemPath
	if err := fs.CopyFile(&fs.LstatCachedFile{Path: fs.ResolveUnknownPath(root, outputPath)}, name); err != nil {
		return err
	}
	return nil
}

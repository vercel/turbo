package runsummary

import (
	"encoding/json"
	"fmt"
	"os"
	"strconv"
	"strings"
	"text/tabwriter"

	"github.com/mitchellh/cli"
	"github.com/vercel/turbo/cli/internal/util"
	"github.com/vercel/turbo/cli/internal/workspace"
)

// FormatAndPrintText prints a Run Summary to the Terminal UI
func (summary RunSummary) FormatAndPrintText(ui cli.Ui, workspaceInfos workspace.Catalog, isSinglePackage bool) error {
	summary.normalize() // normalize data

	if !isSinglePackage {
		ui.Output("")
		ui.Info(util.Sprintf("${CYAN}${BOLD}Packages in Scope${RESET}"))
		p := tabwriter.NewWriter(os.Stdout, 0, 0, 1, ' ', 0)
		fmt.Fprintln(p, "Name\tPath\t")
		for _, pkg := range summary.Packages {
			fmt.Fprintf(p, "%s\t%s\t\n", pkg, workspaceInfos.PackageJSONs[pkg].Dir)
		}
		if err := p.Flush(); err != nil {
			return err
		}
	}

	fileCount := 0
	for range summary.GlobalHashSummary.GlobalFileHashMap {
		fileCount = fileCount + 1
	}
	w1 := tabwriter.NewWriter(os.Stdout, 0, 0, 1, ' ', 0)
	ui.Output("")
	ui.Info(util.Sprintf("${CYAN}${BOLD}Global Hash Inputs${RESET}"))
	fmt.Fprintln(w1, util.Sprintf("  ${GREY}Global Files\t=\t%d${RESET}", fileCount))
	fmt.Fprintln(w1, util.Sprintf("  ${GREY}External Dependencies Hash\t=\t%s${RESET}", summary.GlobalHashSummary.RootExternalDepsHash))
	fmt.Fprintln(w1, util.Sprintf("  ${GREY}Global Cache Key\t=\t%s${RESET}", summary.GlobalHashSummary.GlobalCacheKey))
	if bytes, err := json.Marshal(summary.GlobalHashSummary.Pipeline); err == nil {
		fmt.Fprintln(w1, util.Sprintf("  ${GREY}Root pipeline\t=\t%s${RESET}", bytes))
	}
	if err := w1.Flush(); err != nil {
		return err
	}

	ui.Output("")
	ui.Info(util.Sprintf("${CYAN}${BOLD}Tasks to Run${RESET}"))

	for _, task := range summary.Tasks {
		taskName := task.TaskID

		if isSinglePackage {
			taskName = util.RootTaskTaskName(taskName)
		}

		ui.Info(util.Sprintf("${BOLD}%s${RESET}", taskName))
		w := tabwriter.NewWriter(os.Stdout, 0, 0, 1, ' ', 0)
		fmt.Fprintln(w, util.Sprintf("  ${GREY}Task\t=\t%s\t${RESET}", task.Task))

		var dependencies []string
		var dependents []string

		if !isSinglePackage {
			fmt.Fprintln(w, util.Sprintf("  ${GREY}Package\t=\t%s\t${RESET}", task.Package))
			dependencies = task.Dependencies
			dependents = task.Dependents
		} else {
			dependencies = make([]string, len(task.Dependencies))
			for i, dependency := range task.Dependencies {
				dependencies[i] = util.StripPackageName(dependency)
			}
			dependents = make([]string, len(task.Dependents))
			for i, dependent := range task.Dependents {
				dependents[i] = util.StripPackageName(dependent)
			}
		}

		fmt.Fprintln(w, util.Sprintf("  ${GREY}Hash\t=\t%s\t${RESET}", task.Hash))
		fmt.Fprintln(w, util.Sprintf("  ${GREY}Cached (Local)\t=\t%s\t${RESET}", strconv.FormatBool(task.CacheState.Local)))
		fmt.Fprintln(w, util.Sprintf("  ${GREY}Cached (Remote)\t=\t%s\t${RESET}", strconv.FormatBool(task.CacheState.Remote)))

		if !isSinglePackage {
			fmt.Fprintln(w, util.Sprintf("  ${GREY}Directory\t=\t%s\t${RESET}", task.Dir))
		}

		fmt.Fprintln(w, util.Sprintf("  ${GREY}Command\t=\t%s\t${RESET}", task.Command))
		fmt.Fprintln(w, util.Sprintf("  ${GREY}Outputs\t=\t%s\t${RESET}", strings.Join(task.Outputs, ", ")))
		fmt.Fprintln(w, util.Sprintf("  ${GREY}Log File\t=\t%s\t${RESET}", task.LogFile))
		fmt.Fprintln(w, util.Sprintf("  ${GREY}Dependencies\t=\t%s\t${RESET}", strings.Join(dependencies, ", ")))
		fmt.Fprintln(w, util.Sprintf("  ${GREY}Dependendents\t=\t%s\t${RESET}", strings.Join(dependents, ", ")))
		fmt.Fprintln(w, util.Sprintf("  ${GREY}Inputs Files Considered\t=\t%d\t${RESET}", len(task.ExpandedInputs)))

		fmt.Fprintln(w, util.Sprintf("  ${GREY}Configured Environment Variables\t=\t%s\t${RESET}", strings.Join(task.EnvVars.Configured, ", ")))
		fmt.Fprintln(w, util.Sprintf("  ${GREY}Inferred Environment Variables\t=\t%s\t${RESET}", strings.Join(task.EnvVars.Inferred, ", ")))
		fmt.Fprintln(w, util.Sprintf("  ${GREY}Global Environment Variables\t=\t%s\t${RESET}", strings.Join(task.EnvVars.Global, ", ")))

		bytes, err := json.Marshal(task.ResolvedTaskDefinition)
		// If there's an error, we can silently ignore it, we don't need to block the entire print.
		if err == nil {
			fmt.Fprintln(w, util.Sprintf("  ${GREY}ResolvedTaskDefinition\t=\t%s\t${RESET}", string(bytes)))
		}

		fmt.Fprintln(w, util.Sprintf("  ${GREY}Framework\t=\t%s\t${RESET}", task.Framework))
		if err := w.Flush(); err != nil {
			return err
		}
	}
	return nil
}

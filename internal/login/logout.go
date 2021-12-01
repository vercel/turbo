package login

import (
	"fmt"
	"strings"
	"turbo/internal/config"

	"github.com/fatih/color"
	"github.com/mitchellh/cli"
)

// LogoutCommand is a Command implementation that tells Turbo to run a task
type LogoutCommand struct {
	Config *config.Config
	Ui     *cli.ColoredUi
}

// Synopsis of run command
func (c *LogoutCommand) Synopsis() string {
	return "DEPRECATED - Logout to your Turborepo.com account"
}

// Help returns information about the `run` command
func (c *LogoutCommand) Help() string {
	helpText := `
Usage: turbo logout

  Logout to your Turborepo.com account
`
	return strings.TrimSpace(helpText)
}

// Run executes tasks in the monorepo
func (c *LogoutCommand) Run(args []string) int {
	pref := color.New(color.Bold, color.FgRed, color.ReverseVideo).Sprint(" ERROR ")
	c.Ui.Output(fmt.Sprintf("%s%s", pref, color.RedString(" This command has been deprecated. Please use `turbo unlink` instead.")))
	return 1
}

package nodejs

import (
	"fmt"
	"io/ioutil"
	"path/filepath"

	"github.com/vercel/turborepo/cli/internal/fs"
	"github.com/vercel/turborepo/cli/internal/package_managers/api"
	"gopkg.in/yaml.v2"
)

// PnpmWorkspaces is a representation of workspace package globs found
// in pnpm-workspace.yaml
type PnpmWorkspaces struct {
	Packages []string `yaml:"packages,omitempty"`
}

var NodejsPnpm = api.PackageManager{
	Name:       "nodejs-pnpm",
	Slug:       "pnpm",
	Command:    "pnpm",
	Specfile:   "package.json",
	Lockfile:   "pnpm-lock.yaml",
	PackageDir: "node_modules",

	GetWorkspaceGlobs: func(rootpath string) ([]string, error) {
		bytes, err := ioutil.ReadFile(filepath.Join(rootpath, "pnpm-workspace.yaml"))
		if err != nil {
			return nil, fmt.Errorf("pnpm-workspace.yaml: %w", err)
		}
		var pnpmWorkspaces PnpmWorkspaces
		if err := yaml.Unmarshal(bytes, &pnpmWorkspaces); err != nil {
			return nil, fmt.Errorf("pnpm-workspace.yaml: %w", err)
		}

		if len(pnpmWorkspaces.Packages) == 0 {
			return nil, fmt.Errorf("pnpm-workspace.yaml: no packages found. Turborepo requires PNPM workspaces and thus packages to be defined in the root pnpm-workspace.yaml")
		}

		return pnpmWorkspaces.Packages, nil
	},

	Matches: func(manager string, version string) (bool, error) {
		return manager == "pnpm", nil
	},

	Detect: func(projectDirectory string, pkg *fs.PackageJSON, packageManager *api.PackageManager) (bool, error) {
		specfileExists := fs.FileExists(filepath.Join(projectDirectory, packageManager.Specfile))
		lockfileExists := fs.FileExists(filepath.Join(projectDirectory, packageManager.Lockfile))

		return (specfileExists && lockfileExists), nil
	},
}

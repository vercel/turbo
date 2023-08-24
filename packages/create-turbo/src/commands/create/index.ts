import path from "node:path";
import chalk from "chalk";
import type { Project } from "@turbo/workspaces";
import {
  getWorkspaceDetails,
  install,
  getPackageManagerMeta,
  ConvertError,
} from "@turbo/workspaces";
import {
  getAvailablePackageManagers,
  createProject,
  logger,
} from "@turbo/utils";
import { tryGitCommit, tryGitInit } from "../../utils/git";
import { isOnline } from "../../utils/isOnline";
import { transforms } from "../../transforms";
import { TransformError } from "../../transforms/errors";
import { isDefaultExample } from "../../utils/isDefaultExample";
import * as prompts from "./prompts";
import type { CreateCommandArgument, CreateCommandOptions } from "./types";

const { turboGradient, turboLoader, info, error, warn } = logger;

function handleErrors(err: unknown) {
  // handle errors from ../../transforms
  if (err instanceof TransformError) {
    error(chalk.bold(err.transform), chalk.red(err.message));
    if (err.fatal) {
      process.exit(1);
    }
    // handle errors from @turbo/workspaces
  } else if (err instanceof ConvertError && err.type !== "unknown") {
    error(chalk.red(err.message));
    process.exit(1);
    // handle unknown errors (no special handling, just re-throw to catch at root)
  } else {
    throw err;
  }
}

const SCRIPTS_TO_DISPLAY: Record<string, string> = {
  build: "Build",
  dev: "Develop",
  test: "Test",
  lint: "Lint",
};

export async function create(
  directory: CreateCommandArgument,
  packageManager: CreateCommandArgument,
  opts: CreateCommandOptions
) {
  const { skipInstall, skipTransforms } = opts;
  // eslint-disable-next-line no-console
  console.log(chalk.bold(turboGradient(`\n>>> TURBOREPO\n`)));
  info(`Welcome to Turborepo! Let's get you set up with a new codebase.`);
  // eslint-disable-next-line no-console
  console.log();

  const [online, availablePackageManagers] = await Promise.all([
    isOnline(),
    getAvailablePackageManagers(),
  ]);

  if (!online) {
    error(
      "You appear to be offline. Please check your network connection and try again."
    );
    process.exit(1);
  }
  const { root, projectName } = await prompts.directory({ dir: directory });
  const relativeProjectDir = path.relative(process.cwd(), root);
  const projectDirIsCurrentDir = relativeProjectDir === "";

  // selected package manager can be undefined if the user chooses to skip transforms
  const selectedPackageManagerDetails = await prompts.packageManager({
    manager: packageManager,
    skipTransforms,
  });

  if (packageManager && opts.skipTransforms) {
    warn(
      "--skip-transforms conflicts with <package-manager>. The package manager argument will be ignored."
    );
  }

  const { example, examplePath } = opts;
  const exampleName = example && example !== "default" ? example : "basic";
  const { hasPackageJson, availableScripts, repoInfo } = await createProject({
    appPath: root,
    example: exampleName,
    isDefaultExample: isDefaultExample(exampleName),
    examplePath,
  });

  // create a new git repo after creating the project
  tryGitInit(root, `feat(create-turbo): create ${exampleName}`);

  // read the project after creating it to get details about workspaces, package manager, etc.
  let project: Project = {} as Project;
  try {
    project = await getWorkspaceDetails({ root });
  } catch (err) {
    handleErrors(err);
  }

  // run any required transforms
  if (!skipTransforms) {
    for (const transform of transforms) {
      try {
        // eslint-disable-next-line no-await-in-loop
        const transformResult = await transform({
          example: {
            repo: repoInfo,
            name: exampleName,
          },
          project,
          prompts: {
            projectName,
            root,
            packageManager: selectedPackageManagerDetails,
          },
          opts,
        });
        if (transformResult.result === "success") {
          tryGitCommit(
            `feat(create-turbo): apply ${transformResult.name} transform`
          );
        }
      } catch (err) {
        handleErrors(err);
      }
    }
  }

  // if the user opted out of transforms, the package manager will be the same as the source example
  const projectPackageManager =
    skipTransforms || !selectedPackageManagerDetails
      ? {
          name: project.packageManager,
          version: availablePackageManagers[project.packageManager],
        }
      : selectedPackageManagerDetails;

  info("Created a new Turborepo with the following:");
  // eslint-disable-next-line no-console
  console.log();
  if (project.workspaceData.workspaces.length > 0) {
    const workspacesForDisplay = project.workspaceData.workspaces
      .map((w) => ({
        group: path.relative(root, w.paths.root).split(path.sep)[0] || "",
        title: path.relative(root, w.paths.root),
        description: w.description,
      }))
      .sort((a, b) => a.title.localeCompare(b.title));

    let lastGroup: string | undefined;
    workspacesForDisplay.forEach(({ group, title, description }, idx) => {
      if (idx === 0 || group !== lastGroup) {
        // eslint-disable-next-line no-console
        console.log(chalk.cyan(group));
      }
      // eslint-disable-next-line no-console
      console.log(
        ` - ${chalk.bold(title)}${description ? `: ${description}` : ""}`
      );
      lastGroup = group;
    });
  } else {
    // eslint-disable-next-line no-console
    console.log(chalk.cyan("apps"));
    // eslint-disable-next-line no-console
    console.log(` - ${chalk.bold(projectName)}`);
  }

  // run install
  // eslint-disable-next-line no-console
  console.log();
  if (hasPackageJson && !skipInstall) {
    // in the case when the user opted out of transforms, but not install, we need to make sure the package manager is available
    // before we attempt an install
    if (
      opts.skipTransforms &&
      !availablePackageManagers[project.packageManager]
    ) {
      warn(
        `Unable to install dependencies - "${exampleName}" uses "${project.packageManager}" which could not be found.`
      );
      warn(
        `Try running without "--skip-transforms" to convert "${exampleName}" to a package manager that is available on your system.`
      );
      // eslint-disable-next-line no-console
      console.log();
    } else if (projectPackageManager.version) {
      // eslint-disable-next-line no-console
      console.log("Installing packages. This might take a couple of minutes.");
      // eslint-disable-next-line no-console
      console.log();

      const loader = turboLoader("Installing dependencies...").start();
      await install({
        project,
        to: projectPackageManager,
        options: {
          interactive: false,
        },
      });

      tryGitCommit("feat(create-turbo): install dependencies");
      loader.stop();
    }
  }

  if (projectDirIsCurrentDir) {
    // eslint-disable-next-line no-console
    console.log(
      `${chalk.bold(
        turboGradient(">>> Success!")
      )} Your new Turborepo is ready.`
    );
  } else {
    // eslint-disable-next-line no-console
    console.log(
      `${chalk.bold(
        turboGradient(">>> Success!")
      )} Created a new Turborepo at "${relativeProjectDir}".`
    );
  }

  // get the package manager details so we display the right commands to the user in log messages
  const packageManagerMeta = getPackageManagerMeta(projectPackageManager);
  /* eslint-disable no-console */
  if (packageManagerMeta && hasPackageJson) {
    console.log(
      `Inside ${
        projectDirIsCurrentDir ? "this" : "that"
      } directory, you can run several commands:`
    );
    console.log();
    availableScripts
      .filter((script) => SCRIPTS_TO_DISPLAY[script])
      .forEach((script) => {
        console.log(
          chalk.cyan(`  ${packageManagerMeta.command} run ${script}`)
        );
        console.log(`     ${SCRIPTS_TO_DISPLAY[script]} all apps and packages`);
        console.log();
      });
    console.log(`Turborepo will cache locally by default. For an additional`);
    console.log(`speed boost, enable Remote Caching with Vercel by`);
    console.log(`entering the following command:`);
    console.log();
    console.log(chalk.cyan(`  ${packageManagerMeta.executable} turbo login`));
    console.log();
    console.log(`We suggest that you begin by typing:`);
    console.log();
    if (!projectDirIsCurrentDir) {
      console.log(`  ${chalk.cyan("cd")} ${relativeProjectDir}`);
    }
    console.log(chalk.cyan(`  ${packageManagerMeta.executable} turbo login`));
    console.log();
  }
  /* eslint-enable no-console */
}

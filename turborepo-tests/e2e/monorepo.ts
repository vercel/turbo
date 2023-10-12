import execa from "execa";
import fsNormal from "fs";
import globby from "globby";
import fs from "fs-extra";
import os from "os";
import path from "path";
const isWin = process.platform === "win32";
const turboPath = path.join(
  __dirname,
  "../../target/debug/turbo" + (isWin ? ".exe" : "")
);
import type { PackageManager } from "./types";

const PACKAGE_MANAGER_VERSIONS: { [key in PackageManager]: string } = {
  yarn: "yarn@1.22.17",
  berry: "yarn@3.1.1",
  pnpm6: "pnpm@6.22.2",
  pnpm: "pnpm@7.2.1",
  npm: "npm@8.3.0",
};

interface MonorepoOptions {
  root: string;
  pm: PackageManager;
  pipeline: any;
  subdir?: string;
}

export class Monorepo {
  static tmpdir = os.tmpdir();
  static yarnCache = path.join(__dirname, "yarn-cache-");
  root: string;
  subdir?: string;
  turboConfig: any;
  name: string;
  npmClient: PackageManager;

  get nodeModulesPath() {
    return this.subdir
      ? path.join(this.root, this.subdir, "node_modules")
      : path.join(this.root, "node_modules");
  }

  constructor(options: MonorepoOptions) {
    this.root = fs.mkdtempSync(
      path.join(__dirname, `turbo-monorepo-${options.root}-`)
    );
    this.npmClient = options.pm;
    this.turboConfig = options.pipeline;
    this.subdir = options.subdir;
  }

  init() {
    fs.removeSync(path.join(this.root, ".git"));
    fs.ensureDirSync(path.join(this.root, ".git"));

    if (this.subdir) {
      fs.ensureDirSync(path.join(this.root, this.subdir));
    }

    fs.writeFileSync(
      path.join(this.root, ".git", "config"),
      `
  [user]
    name = GitHub Actions
    email = actions@users.noreply.github.com

  [init]
    defaultBranch = main
  `
    );
    execa.sync("git", ["init", "-q"], { cwd: this.root });
    this.generateRepoFiles(this.turboConfig);
  }

  install() {
    if (!fs.existsSync(this.nodeModulesPath)) {
      fs.mkdirSync(this.nodeModulesPath, { recursive: true });
    }
  }

  /**
   * Simulates a "yarn" call by linking internal packages and generates a yarn.lock file
   */
  _linkNpmPackages() {
    const cwd = this.subdir ? path.join(this.root, this.subdir) : this.root;

    if (!fs.existsSync(this.nodeModulesPath)) {
      fs.mkdirSync(this.nodeModulesPath, { recursive: true });
    }

    const data = fsNormal.readFileSync(`${cwd}/package.json`, "utf8");

    const pkg = JSON.parse(data.toString());
    pkg.packageManager = PACKAGE_MANAGER_VERSIONS[this.npmClient];

    fsNormal.writeFileSync(`${cwd}/package.json`, JSON.stringify(pkg, null, 2));
    // Ensure that the package.json file is committed
    this.commitAll();

    if (this.npmClient == "npm") {
      execa.sync("npm", ["install"], {
        cwd,
      });
      this.commitAll();
      return;
    }
  }

  /**
   * Simulates a "yarn" call by linking internal packages and generates a yarn.lock file
   */
  linkPackages() {
    if (this.npmClient == "npm") {
      this._linkNpmPackages();
      return;
    }

    const cwd = this.subdir ? path.join(this.root, this.subdir) : this.root;
    const pkgs = fs.readdirSync(path.join(cwd, "packages"));

    if (!fs.existsSync(this.nodeModulesPath)) {
      fs.mkdirSync(this.nodeModulesPath, { recursive: true });
    }

    const data = fsNormal.readFileSync(`${cwd}/package.json`, "utf8");

    const pkg = JSON.parse(data.toString());
    pkg.packageManager = PACKAGE_MANAGER_VERSIONS[this.npmClient];

    fsNormal.writeFileSync(`${cwd}/package.json`, JSON.stringify(pkg, null, 2));
    // Ensure that the package.json file is committed
    this.commitAll();

    let yarnYaml = `# THIS IS AN AUTOGENERATED FILE. DO NOT EDIT THIS FILE DIRECTLY.\n# yarn lockfile v1\n`;

    if (this.npmClient == "pnpm6" || this.npmClient == "pnpm") {
      this.commitFiles({
        "pnpm-workspace.yaml": `packages:
- packages/*`,
        "pnpm-lock.yaml": `lockfileVersion: ${
          this.npmClient == "pnpm6" ? 5.3 : 5.4
        }

importers:

  .:
    specifiers: {}

  packages/a:
    specifiers:
      b: workspace:*
    dependencies:
      b: link:../b

  packages/b:
    specifiers: {}

  packages/c:
    specifiers: {}${this.npmClient == "pnpm6" ? "" : "\n"}`,
      });
      execa.sync("pnpm", ["install", "--recursive"], {
        cwd,
      });
      return;
    }

    for (const pkg of pkgs) {
      fs.symlinkSync(
        path.join(cwd, "packages", pkg),
        path.join(this.nodeModulesPath, pkg),
        "junction"
      );

      if (this.npmClient == "yarn" || this.npmClient == "berry") {
        const pkgJson = JSON.parse(
          fs.readFileSync(
            path.join(cwd, "packages", pkg, "package.json"),
            "utf-8"
          )
        );
        const deps = pkgJson.dependencies;

        yarnYaml += `\n"${pkg}@^${pkgJson.version}":\n  version "${pkgJson.version}"\n`;

        if (deps && Object.keys(deps).length > 0) {
          yarnYaml += `  dependencies:\n`;
          for (const dep of Object.keys(deps)) {
            yarnYaml += `    "${dep}" "0.1.0"\n`;
          }
        }
        this.commitFiles({ "yarn.lock": yarnYaml });

        if (this.npmClient == "berry") {
          execa.sync("yarn", ["install"], {
            cwd,
            env: {
              YARN_ENABLE_IMMUTABLE_INSTALLS: "false",
            },
          });
          this.commitAll();
          return;
        }
      }
    }
  }

  generateRepoFiles(turboConfig = {}) {
    this.commitFiles({
      [`.gitignore`]: `node_modules\n.turbo\n!*-lock.json\ndist/\nout/\n`,
      "package.json": {
        name: this.name,
        version: "0.1.0",
        private: true,
        license: "MIT",
        workspaces: ["packages/**"],
        scripts: {
          build: `echo building`,
          test: `${turboPath} run test`,
          lint: `${turboPath} run lint`,
          special: "echo root task",
          // We have a trailing '--' as node swallows the first '--'
          // We prepend the output with Output to make finding the script output
          // easier during testing.
          args: "node -e \"console.log('Output: ' + JSON.stringify(process.argv))\" --",
        },
      },
      "turbo.json": {
        ...turboConfig,
      },
    });
  }

  addPackage(name, internalDeps: string[] = []) {
    return this.commitFiles({
      [`packages/${name}/build.js`]: `
const fs = require('fs');
const path = require('path');
console.log('building ${name}');

if (!fs.existsSync(path.join(__dirname, 'dist'))){
  fs.mkdirSync(path.join(__dirname, 'dist'));
}

fs.copyFileSync(
  path.join(__dirname, 'build.js'),
  path.join(__dirname, 'dist', 'build.js')
);
`,
      [`packages/${name}/test.js`]: `console.log('testing ${name}');`,
      [`packages/${name}/lint.js`]: `console.log('linting ${name}');`,
      [`packages/${name}/package.json`]: {
        name,
        version: "0.1.0",
        license: "MIT",
        scripts: {
          build: "node ./build.js",
          test: "node ./test.js",
          lint: "node ./lint.js",
        },
        dependencies: {
          ...(internalDeps &&
            internalDeps.reduce((deps, dep) => {
              return {
                ...deps,
                [dep]:
                  this.npmClient === "pnpm" ||
                  this.npmClient === "pnpm6" ||
                  this.npmClient === "berry"
                    ? "workspace:*"
                    : "*",
              };
            }, {})),
        },
      },
    });
  }

  clone(origin) {
    return execa.sync("git", ["clone", origin], { cwd: this.root });
  }

  push(origin, branch) {
    return execa.sync("git", ["push", origin, branch], { cwd: this.root });
  }

  newBranch(branch) {
    return execa.sync("git", ["checkout", "-B", branch], { cwd: this.root });
  }

  modifyFiles(files: { [filename: string]: string }) {
    for (const [file, contents] of Object.entries(files)) {
      let out = "";
      if (typeof contents !== "string") {
        out = JSON.stringify(contents, null, 2);
      } else {
        out = contents;
      }

      const fullPath =
        this.subdir != null
          ? path.join(this.root, this.subdir, file)
          : path.join(this.root, file);

      if (!fs.existsSync(path.dirname(fullPath))) {
        fs.mkdirSync(path.dirname(fullPath), { recursive: true });
      }

      fs.writeFileSync(fullPath, out);
    }
  }

  commitFiles(files) {
    this.modifyFiles(files);
    execa.sync(
      "git",
      [
        "add",
        ...Object.keys(files).map((f) =>
          this.subdir != null
            ? path.join(this.root, this.subdir, f)
            : path.join(this.root, f)
        ),
      ],
      {
        cwd: this.root,
      }
    );
    return execa.sync("git", ["commit", "-m", "foo"], {
      cwd: this.root,
    });
  }

  commitAll() {
    execa.sync("git", ["add", "."], {
      cwd: this.root,
    });
    return execa.sync("git", ["commit", "-m", "foo"], {
      cwd: this.root,
    });
  }

  expectCleanGitStatus() {
    const status = execa.sync("git", ["status", "-s"], {
      cwd: this.root,
    });
    if (status.stdout !== "" || status.stderr !== "") {
      throw new Error(
        `Found git status: stdout ${status.stdout} / stderr ${status.stderr}`
      );
    }
  }

  turbo(
    command,
    args?: readonly string[],
    options?: execa.SyncOptions<string>
  ) {
    const resolvedArgs = [...args];

    return execa.sync(turboPath, [command, ...resolvedArgs], {
      cwd: this.root,
      shell: true,
      ...options,
    });
  }

  run(command, args?: readonly string[], options?: execa.SyncOptions<string>) {
    switch (this.npmClient) {
      case "yarn":
        return execa.sync("yarn", [command, ...(args || [])], {
          cwd: this.root,
          shell: true,
          ...options,
        });
      case "berry":
        return execa.sync("yarn", [command, ...(args || [])], {
          cwd: this.root,
          shell: true,
          ...options,
        });
      case "pnpm6":
      case "pnpm":
        return execa.sync("pnpm", [command, ...(args || [])], {
          cwd: this.root,
          shell: true,
          ...options,
        });
      case "npm":
        return execa.sync("npm", [command, ...(args || [])], {
          cwd: this.root,
          shell: true,
          ...options,
        });
      default:
        throw new Error("npm client not implemented yet");
    }
  }

  readFileSync(filepath) {
    return fs.readFileSync(path.join(this.root, filepath), "utf-8");
  }

  readdirSync(filepath) {
    return fs.readdirSync(path.join(this.root, filepath), "utf-8");
  }

  globbySync(
    patterns: string | readonly string[],
    options?: globby.GlobbyOptions
  ) {
    return globby.sync(patterns, { cwd: this.root, ...options });
  }

  async globby(
    patterns: string | readonly string[],
    options?: globby.GlobbyOptions
  ) {
    return await globby(patterns, { cwd: this.root, ...options });
  }

  cleanup() {
    fs.rmSync(this.root, { recursive: true });
  }
}

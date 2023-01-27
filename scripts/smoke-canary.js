#!/usr/bin/env node

const { execSync } = require("child_process");
const { assert } = require("console");

function exec({ command, options }) {
  console.log(`Running: "${command}"`);
  try {
    const result = execSync(command, options).toString();
    if (process.env.GITHUB_ACTIONS === "true") {
      console.log(`::group::"${command}" output`);
      console.log(result);
      console.log(`::endgroup::`);
    } else {
      console.log(result);
    }

    return result;
  } catch (err) {
    console.error(err);
    console.error(err.stdout.toString());
    process.exit(1);
  }
}

function installGlobalTurbo({ packageManager }) {
  console.log("Install global turbo");
  if (packageManager === "pnpm" || packageManager === "npm") {
    exec({ command: `${packageManager} install turbo --global` });
  } else {
    exec({ command: `${packageManager} global add turbo` });
  }
}

function uninstallLocalTurbo({ packageManager }) {
  console.log("Uninstall local turbo");
  if (packageManager === "pnpm" || packageManager === "npm") {
    exec({ command: `${packageManager} uninstall turbo` });
  } else {
    exec({ command: `${packageManager} remove turbo` });
  }
}

function local({ version, packageManager }) {
  const createTurboOutput = exec({
    command: `npx create-turbo@${version} --help --use-${packageManager} .`,
  });
  assert(createTurboOutput.includes("Success! Your new Turborepo is ready."));

  console.log("Turbo details");
  exec({ command: `${packageManager} turbo --version` });
  exec({ command: `${packageManager} turbo bin` });

  console.log("Verify binary is not global");
  const turboBin = exec({ command: `${packageManager} turbo bin` });
  assert(!turboBin.includes("global"));

  console.log("Verify turbo build");
  const turboFirstBuildOutput = exec({
    command: `${packageManager} turbo build`,
  });
  assert(turboFirstBuildOutput.includes("2 successful, 2 total"));
  assert(turboFirstBuildOutput.includes("0 cached, 2 total"));
  assert(!turboFirstBuildOutput.includes("FULL_TURBO"));

  console.log("Verify turbo build (cached)");
  const turboSecondBuildOutput = exec({
    command: `${packageManager} turbo build`,
  });
  assert(turboSecondBuildOutput.includes("2 successful, 2 total"));
  assert(turboSecondBuildOutput.includes("2 cached, 2 total"));
  assert(turboSecondBuildOutput.includes("FULL TURBO"));
}

function global({ version, packageManager }) {
  const createTurboOutput = exec({
    command: `npx create-turbo@${version} --help --use-${packageManager} .`,
  });
  assert(createTurboOutput.includes("Success! Your new Turborepo is ready."));

  installGlobalTurbo({ packageManager });

  console.log("Turbo details");
  exec({ command: `turbo --version` });
  exec({ command: `turbo bin` });

  console.log("Verify binary is not global");
  const turboFirstBin = exec({ command: `turbo bin` });
  assert(!turboFirstBin.includes("global"));

  uninstallLocalTurbo({ packageManager });

  console.log("Turbo details");
  exec({ command: `turbo --version` });
  exec({ command: `turbo bin` });

  console.log("Verify binary is global");
  const turboSecondBin = exec({ command: `turbo bin` });
  assert(turboSecondBin.includes("global"));

  console.log("Verify turbo build");
  const turboFirstBuildOutput = exec({ command: `turbo build` });
  assert(turboFirstBuildOutput.includes("2 successful, 2 total"));
  assert(turboFirstBuildOutput.includes("0 cached, 2 total"));
  assert(!turboFirstBuildOutput.includes("FULL_TURBO"));

  console.log("Verify turbo build (cached)");
  const turboSecondBuildOutput = exec({ command: `turbo build` });
  assert(turboSecondBuildOutput.includes("2 successful, 2 total"));
  assert(turboSecondBuildOutput.includes("2 cached, 2 total"));
  assert(turboSecondBuildOutput.includes("FULL TURBO"));
}

function both({ version, packageManager }) {
  const createTurboOutput = exec({
    command: `npx create-turbo@${version} --help --use-${packageManager} .`,
  });
  assert(createTurboOutput.includes("Success! Your new Turborepo is ready."));

  installGlobalTurbo({ packageManager });

  console.log("Turbo details");
  exec({ command: `turbo --version` });
  exec({ command: `turbo bin` });

  console.log("Verify binary is not global");
  const turboFirstBin = exec({ command: `turbo bin` });
  assert(!turboFirstBin.includes("global"));

  console.log("Verify turbo build");
  const turboFirstBuildOutput = exec({ command: `turbo build` });
  assert(turboFirstBuildOutput.includes("2 successful, 2 total"));
  assert(turboFirstBuildOutput.includes("0 cached, 2 total"));
  assert(!turboFirstBuildOutput.includes("FULL_TURBO"));

  console.log("Verify turbo build (cached)");
  const turboSecondBuildOutput = exec({ command: `turbo build` });
  assert(turboSecondBuildOutput.includes("2 successful, 2 total"));
  assert(turboSecondBuildOutput.includes("2 cached, 2 total"));
  assert(turboSecondBuildOutput.includes("FULL TURBO"));
}

const tests = {
  local,
  global,
  both,
};

function test() {
  const args = process.argv.slice(2);
  const [testName = "local", version = "canary", packageManager = "pnpm"] =
    args;

  console.log(
    `Running test: "${testName}" with version: "turbo@${version}" using ${packageManager}`
  );
  tests[testName]({ version, packageManager });
  console.log("Tests passed!");
}

test();

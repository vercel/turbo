import addDefaultOutputs from "../src/transforms/add-default-outputs";
import path from "path";
import fs from "fs-extra";

const FIXTURES_DIR = path.join(__dirname, "__fixtures__");
let restoreFn = () => {};
const CREATED_DIRS: string[] = [];

function useFixture(dir: string) {
  let copiedDirPath = path.join(
    __dirname,
    "test-runtime",
    `${path.basename(dir)}-${Date.now()}`
  ); // should be __tests__

  // copy the dir to another place
  fs.copySync(dir, copiedDirPath);
  // and store a reference so we can clean up later
  CREATED_DIRS.push(copiedDirPath);

  const cleanupFn = () => {
    fs.rmSync(copiedDirPath, { recursive: true, force: true });
  };

  // return a function that can be called later to restore the original
  return [copiedDirPath, cleanupFn];
}

describe("add-default-outputs", () => {
  beforeEach(() => {
    restoreFn = () => {};
  });

  afterEach(() => {
    restoreFn();
  });

  // clean up all created dirs
  afterAll(() => {
    for (const dir of CREATED_DIRS) {
      fs.rmSync(dir, { recursive: true, force: true });
    }
  });

  test("basic run", () => {
    const fixture = path.join(FIXTURES_DIR, "thingie");
    const [testDir, cleanupFn] = useFixture(fixture);

    const flags = {
      dry: false,
      force: false,
      print: false,
    };

    /* @ts-ignore-next-line */
    addDefaultOutputs([testDir], flags);

    /* @ts-ignore-next-line */
    const output = fs.readJSONSync(path.join(testDir, "turbo.json"));

    expect(output.pipeline["build-one"].outputs).toStrictEqual(["foo"]);
    expect(output.pipeline["build-two"].outputs).toStrictEqual(undefined);
    expect(output.pipeline["build-three"].outputs).toStrictEqual([
      "dist/**",
      "build/**",
    ]);

    // @ts-ignore-next-line
    cleanupFn();
  });

  test("dry run", () => {
    const fixture = path.join(FIXTURES_DIR, "thingie");
    const [testDir, cleanupFn] = useFixture(fixture);

    // @ts-ignore
    addDefaultOutputs([testDir], {
      dry: true,
      force: false,
      print: false,
    });

    // @ts-ignore
    const output = fs.readJSONSync(path.join(testDir, "turbo.json"));

    expect(output.pipeline["build-one"].outputs).toStrictEqual(["foo"]);
    expect(output.pipeline["build-two"].outputs).toStrictEqual([]);
    expect(output.pipeline["build-three"].outputs).toStrictEqual(undefined);

    // @ts-ignore-next-line
    cleanupFn();
  });
});

import getCodemodsForMigration from "../src/commands/migrate/steps/getTransformsForMigration";

describe("get-transforms-for-migration", () => {
  test("ordering", () => {
    let results = getCodemodsForMigration({
      fromVersion: "1.0.0",
      toVersion: "1.10.0",
    });

    expect(results.map((transform) => transform.value)).toEqual([
      "add-package-manager",
      "create-turbo-config",
      "migrate-env-var-dependencies",
      "set-default-outputs",
      "stabilize-env-mode",
      "transform-literals-to-wildcards",
    ]);
  });
});

import "./dir";
import "package-with-exports/entry";

it("should not bundle the root level package", () => {
  const modules = Object.keys(__turbopack_modules__);
  expect(modules).toContainEqual(
    expect.stringMatching(/input\/dir\/node_modules\/the-package\/index/)
  );
  expect(modules).not.toContainEqual(
    expect.stringMatching(/input\/node_modules\/the-package\/index/)
  );
});

it("should not bundle the other exports conditions", () => {
  require("package-with-exports/entry2");
  const modules = Object.keys(__turbopack_modules__);
  expect(modules).toContainEqual(
    expect.stringMatching(/input\/node_modules\/package-with-exports\/a/)
  );
  expect(modules).not.toContainEqual(
    expect.stringMatching(/input\/node_modules\/package-with-exports\/index/)
  );
  expect(modules).not.toContainEqual(
    expect.stringMatching(/input\/node_modules\/package-with-exports\/b/)
  );
  expect(modules).not.toContainEqual(
    expect.stringMatching(/input\/node_modules\/package-with-exports\/c/)
  );
  expect(modules).not.toContainEqual(
    expect.stringMatching(/input\/node_modules\/package-with-exports\/entry/)
  );
  expect(modules).not.toContainEqual(
    expect.stringMatching(/input\/node_modules\/package-with-exports\/entry2/)
  );
  expect(modules).not.toContainEqual(
    expect.stringMatching(/input\/node_modules\/package-with-exports\/main/)
  );
  expect(modules).not.toContainEqual(
    expect.stringMatching(/input\/node_modules\/package-with-exports\/module/)
  );
});

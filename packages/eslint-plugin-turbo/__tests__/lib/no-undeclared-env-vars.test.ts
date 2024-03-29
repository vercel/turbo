import path from "node:path";
import { RuleTester } from "eslint";
import { RULES } from "../../lib/constants";
import rule from "../../lib/rules/no-undeclared-env-vars";

const ruleTester = new RuleTester({
  parserOptions: { ecmaVersion: 2020 },
});

const moduleRuleTester = new RuleTester({
  parserOptions: { ecmaVersion: 2020, sourceType: "module" },
});

ruleTester.run(RULES.noUndeclaredEnvVars, rule, {
  valid: [
    {
      code: `
        const env2 = process.env['ENV_2'];
      `,
      options: [
        { cwd: path.join(__dirname, "../../__fixtures__/workspace-configs") },
      ],
      filename: path.join(
        __dirname,
        "../../__fixtures__/workspace-configs/apps/web/index.js"
      ),
    },
    {
      code: `
        const env2 = process.env["ENV_2"];
      `,
      options: [
        { cwd: path.join(__dirname, "../../__fixtures__/workspace-configs") },
      ],
      filename: path.join(
        __dirname,
        "../../__fixtures__/workspace-configs/apps/web/index.js"
      ),
    },
    {
      code: `
        const { ENV_2 } = process.env;
      `,
      options: [
        { cwd: path.join(__dirname, "../../__fixtures__/workspace-configs") },
      ],
      filename: path.join(
        __dirname,
        "../../__fixtures__/workspace-configs/apps/web/index.js"
      ),
    },
    {
      code: `
        const { ROOT_DOT_ENV, WEB_DOT_ENV } = process.env;
      `,
      options: [
        { cwd: path.join(__dirname, "../../__fixtures__/workspace-configs") },
      ],
      filename: path.join(
        __dirname,
        "../../__fixtures__/workspace-configs/apps/web/index.js"
      ),
    },
    {
      code: `
        const { NEXT_PUBLIC_HAHAHAHA } = process.env;
      `,
      options: [
        { cwd: path.join(__dirname, "../../__fixtures__/workspace-configs") },
      ],
      filename: path.join(
        __dirname,
        "../../__fixtures__/workspace-configs/apps/web/index.js"
      ),
    },
    {
      code: `
        const { ENV_1 } = process.env;
      `,
      options: [
        { cwd: path.join(__dirname, "../../__fixtures__/workspace-configs") },
      ],
      filename: path.join(
        __dirname,
        "../../__fixtures__/workspace-configs/apps/web/index.js"
      ),
    },
    {
      code: `
        const { ENV_1 } = process.env;
      `,
      options: [{ cwd: "/some/random/path" }],
    },
    {
      code: `
        const { CI } = process.env;
      `,
      options: [
        { cwd: path.join(__dirname, "../../__fixtures__/workspace-configs") },
      ],
      filename: path.join(
        __dirname,
        "../../__fixtures__/workspace-configs/apps/web/index.js"
      ),
    },
    {
      code: `
        const { TASK_ENV_KEY, ANOTHER_ENV_KEY } = process.env;
      `,
      options: [
        { cwd: path.join(__dirname, "../../__fixtures__/configs/single") },
      ],
    },
    {
      code: `
        const { NEW_STYLE_ENV_KEY, TASK_ENV_KEY } = process.env;
      `,
      options: [
        { cwd: path.join(__dirname, "../../__fixtures__/configs/single") },
      ],
    },
    {
      code: `
        const { NEW_STYLE_GLOBAL_ENV_KEY, TASK_ENV_KEY } = process.env;
      `,
      options: [
        { cwd: path.join(__dirname, "../../__fixtures__/configs/single") },
      ],
    },
    {
      code: `
        const val = process.env["NEW_STYLE_GLOBAL_ENV_KEY"];
      `,
      options: [
        { cwd: path.join(__dirname, "../../__fixtures__/configs/single") },
      ],
    },
    {
      code: `
        const { TASK_ENV_KEY, ANOTHER_ENV_KEY } = process.env;
      `,
      options: [
        { cwd: path.join(__dirname, "../../__fixtures__/configs/single") },
      ],
    },
    {
      code: `
        const x = process.env.GLOBAL_ENV_KEY;
        const { TASK_ENV_KEY, GLOBAL_ENV_KEY: renamedX } = process.env;
      `,
      options: [
        { cwd: path.join(__dirname, "../../__fixtures__/configs/single") },
      ],
    },
    {
      code: "var x = process.env.GLOBAL_ENV_KEY;",
      options: [
        { cwd: path.join(__dirname, "../../__fixtures__/configs/single") },
      ],
    },
    {
      code: "let x = process.env.TASK_ENV_KEY;",
      options: [
        { cwd: path.join(__dirname, "../../__fixtures__/configs/single") },
      ],
    },
    {
      code: "const x = process.env.ANOTHER_KEY_VALUE;",
      options: [
        {
          cwd: path.join(__dirname, "../../__fixtures__/configs/single"),
          allowList: ["^ANOTHER_KEY_[A-Z]+$"],
        },
      ],
    },
    {
      code: `
        var x = process.env.ENV_VAR_ONE;
        var y = process.env.ENV_VAR_TWO;
      `,
      options: [
        {
          cwd: path.join(__dirname, "../../__fixtures__/configs/single"),
          allowList: ["^ENV_VAR_[A-Z]+$"],
        },
      ],
    },
    {
      code: `
        var x = process.env.ENV_VAR_ONE;
        var y = process.env.ENV_VAR_TWO;
      `,
      options: [
        {
          cwd: path.join(__dirname, "../../__fixtures__/configs/single"),
          allowList: ["^ENV_VAR_O[A-Z]+$", "ENV_VAR_TWO"],
        },
      ],
    },
    {
      code: `
        var globalOrTask = process.env.TASK_ENV_KEY || process.env.GLOBAL_ENV_KEY;
        var oneOrTwo = process.env.ENV_VAR_ONE || process.env.ENV_VAR_TWO;
      `,
      options: [
        {
          cwd: path.join(__dirname, "../../__fixtures__/configs/single"),
          allowList: ["^ENV_VAR_[A-Z]+$"],
        },
      ],
    },
    {
      code: `
        () => { return process.env.GLOBAL_ENV_KEY }
        () => { return process.env.TASK_ENV_KEY }
        () => { return process.env.ENV_VAR_ALLOWED }
      `,
      options: [
        {
          cwd: path.join(__dirname, "../../__fixtures__/configs/single"),
          allowList: ["^ENV_VAR_[A-Z]+$"],
        },
      ],
    },
    {
      code: `
        var foo = process?.env.GLOBAL_ENV_KEY
        var foo = process?.env.TASK_ENV_KEY
        var foo = process?.env.ENV_VAR_ALLOWED
      `,
      options: [
        {
          cwd: path.join(__dirname, "../../__fixtures__/configs/single"),
          allowList: ["^ENV_VAR_[A-Z]+$"],
        },
      ],
    },
    {
      code: `
        function test(arg1 = process.env.GLOBAL_ENV_KEY) {};
        function test(arg1 = process.env.TASK_ENV_KEY) {};
        function test(arg1 = process.env.ENV_VAR_ALLOWED) {};
      `,
      options: [
        {
          cwd: path.join(__dirname, "../../__fixtures__/configs/single"),
          allowList: ["^ENV_VAR_[A-Z]+$"],
        },
      ],
    },
    {
      code: `
        (arg1 = process.env.GLOBAL_ENV_KEY) => {}
        (arg1 = process.env.TASK_ENV_KEY) => {}
        (arg1 = process.env.ENV_VAR_ALLOWED) => {}
      `,
      options: [
        {
          cwd: path.join(__dirname, "../../__fixtures__/configs/single"),
          allowList: ["^ENV_VAR_[A-Z]+$"],
        },
      ],
    },
    {
      code: "const getEnv = (key) => process.env[key];",
      options: [
        { cwd: path.join(__dirname, "../../__fixtures__/configs/single") },
      ],
    },
    {
      code: "function getEnv(key) { return process.env[key]; }",
      options: [
        { cwd: path.join(__dirname, "../../__fixtures__/configs/single") },
      ],
    },
    {
      code: "for (let x of ['ONE', 'TWO', 'THREE']) { console.log(process.env[x]); }",
      options: [
        { cwd: path.join(__dirname, "../../__fixtures__/configs/single") },
      ],
    },
  ],

  invalid: [
    {
      code: `
        const env2 = process.env['ENV_3'];
      `,
      options: [
        { cwd: path.join(__dirname, "../../__fixtures__/workspace-configs") },
      ],
      filename: path.join(
        __dirname,
        "../../__fixtures__/workspace-configs/apps/web/index.js"
      ),
      errors: [
        {
          message:
            "ENV_3 is not listed as a dependency in the root turbo.json or workspace (apps/web) turbo.json",
        },
      ],
    },
    {
      code: `
        const env2 = process.env["ENV_3"];
      `,
      options: [
        { cwd: path.join(__dirname, "../../__fixtures__/workspace-configs") },
      ],
      filename: path.join(
        __dirname,
        "../../__fixtures__/workspace-configs/apps/web/index.js"
      ),
      errors: [
        {
          message:
            "ENV_3 is not listed as a dependency in the root turbo.json or workspace (apps/web) turbo.json",
        },
      ],
    },
    {
      code: `
        const { ENV_2 } = process.env;
      `,
      options: [
        { cwd: path.join(__dirname, "../../__fixtures__/workspace-configs") },
      ],
      filename: path.join(
        __dirname,
        "../../__fixtures__/workspace-configs/apps/docs/index.js"
      ),
      errors: [
        {
          message:
            "ENV_2 is not listed as a dependency in the root turbo.json or workspace (apps/docs) turbo.json",
        },
      ],
    },
    {
      code: `
        const { NEXT_PUBLIC_HAHAHAHA, NEXT_PUBLIC_EXCLUDE, NEXT_PUBLIC_EXCLUDED } = process.env;
      `,
      options: [
        { cwd: path.join(__dirname, "../../__fixtures__/workspace-configs") },
      ],
      filename: path.join(
        __dirname,
        "../../__fixtures__/workspace-configs/apps/web/index.js"
      ),
      errors: [
        {
          message:
            "NEXT_PUBLIC_EXCLUDE is not listed as a dependency in the root turbo.json or workspace (apps/web) turbo.json",
        },
        {
          message:
            "NEXT_PUBLIC_EXCLUDED is not listed as a dependency in the root turbo.json or workspace (apps/web) turbo.json",
        },
      ],
    },
    {
      code: "let { X } = process.env;",
      options: [
        { cwd: path.join(__dirname, "../../__fixtures__/configs/single") },
      ],
      errors: [{ message: "X is not listed as a dependency in turbo.json" }],
    },
    {
      code: "const { X, Y, Z } = process.env;",
      options: [
        { cwd: path.join(__dirname, "../../__fixtures__/configs/single") },
      ],
      errors: [
        { message: "X is not listed as a dependency in turbo.json" },
        { message: "Y is not listed as a dependency in turbo.json" },
        { message: "Z is not listed as a dependency in turbo.json" },
      ],
    },
    {
      code: "const { X, Y: NewName, Z } = process.env;",
      options: [
        { cwd: path.join(__dirname, "../../__fixtures__/configs/single") },
      ],
      errors: [
        { message: "X is not listed as a dependency in turbo.json" },
        { message: "Y is not listed as a dependency in turbo.json" },
        { message: "Z is not listed as a dependency in turbo.json" },
      ],
    },
    {
      code: "var x = process.env.NOT_THERE;",
      options: [
        { cwd: path.join(__dirname, "../../__fixtures__/configs/single") },
      ],
      errors: [
        {
          message: "NOT_THERE is not listed as a dependency in turbo.json",
        },
      ],
    },
    {
      code: "var x = process.env.KEY;",
      options: [
        {
          cwd: path.join(__dirname, "../../__fixtures__/configs/single"),
          allowList: ["^ANOTHER_KEY_[A-Z]+$"],
        },
      ],
      errors: [{ message: "KEY is not listed as a dependency in turbo.json" }],
    },
    {
      code: `
        var globalOrTask = process.env.TASK_ENV_KEY_NEW || process.env.GLOBAL_ENV_KEY_NEW;
        var oneOrTwo = process.env.ENV_VAR_ONE || process.env.ENV_VAR_TWO;
      `,
      options: [
        {
          cwd: path.join(__dirname, "../../__fixtures__/configs/single"),
        },
      ],
      errors: [
        {
          message:
            "TASK_ENV_KEY_NEW is not listed as a dependency in turbo.json",
        },
        {
          message:
            "GLOBAL_ENV_KEY_NEW is not listed as a dependency in turbo.json",
        },
        {
          message: "ENV_VAR_ONE is not listed as a dependency in turbo.json",
        },
        {
          message: "ENV_VAR_TWO is not listed as a dependency in turbo.json",
        },
      ],
    },
    {
      code: `
        () => { return process.env.GLOBAL_ENV_KEY_NEW }
        () => { return process.env.TASK_ENV_KEY_NEW }
        () => { return process.env.ENV_VAR_NOT_ALLOWED }
      `,
      options: [
        {
          cwd: path.join(__dirname, "../../__fixtures__/configs/single"),
        },
      ],
      errors: [
        {
          message:
            "GLOBAL_ENV_KEY_NEW is not listed as a dependency in turbo.json",
        },
        {
          message:
            "TASK_ENV_KEY_NEW is not listed as a dependency in turbo.json",
        },
        {
          message:
            "ENV_VAR_NOT_ALLOWED is not listed as a dependency in turbo.json",
        },
      ],
    },
    {
      code: `
        var foo = process?.env.GLOBAL_ENV_KEY_NEW
        var foo = process?.env.TASK_ENV_KEY_NEW
        var foo = process?.env.ENV_VAR_NOT_ALLOWED
      `,
      options: [
        {
          cwd: path.join(__dirname, "../../__fixtures__/configs/single"),
        },
      ],
      errors: [
        {
          message:
            "GLOBAL_ENV_KEY_NEW is not listed as a dependency in turbo.json",
        },
        {
          message:
            "TASK_ENV_KEY_NEW is not listed as a dependency in turbo.json",
        },
        {
          message:
            "ENV_VAR_NOT_ALLOWED is not listed as a dependency in turbo.json",
        },
      ],
    },
    {
      code: `
        function test(arg1 = process.env.GLOBAL_ENV_KEY_NEW) {};
        function test(arg1 = process.env.TASK_ENV_KEY_NEW) {};
        function test(arg1 = process.env.ENV_VAR_NOT_ALLOWED) {};
      `,
      options: [
        {
          cwd: path.join(__dirname, "../../__fixtures__/configs/single"),
        },
      ],
      errors: [
        {
          message:
            "GLOBAL_ENV_KEY_NEW is not listed as a dependency in turbo.json",
        },
        {
          message:
            "TASK_ENV_KEY_NEW is not listed as a dependency in turbo.json",
        },
        {
          message:
            "ENV_VAR_NOT_ALLOWED is not listed as a dependency in turbo.json",
        },
      ],
    },
    {
      code: `
        (arg1 = process.env.GLOBAL_ENV_KEY_NEW) => {}
        (arg1 = process.env.TASK_ENV_KEY_NEW) => {}
        (arg1 = process.env.ENV_VAR_NOT_ALLOWED) => {}
      `,
      options: [
        {
          cwd: path.join(__dirname, "../../__fixtures__/configs/single"),
        },
      ],
      errors: [
        {
          message:
            "GLOBAL_ENV_KEY_NEW is not listed as a dependency in turbo.json",
        },
        {
          message:
            "TASK_ENV_KEY_NEW is not listed as a dependency in turbo.json",
        },
        {
          message:
            "ENV_VAR_NOT_ALLOWED is not listed as a dependency in turbo.json",
        },
      ],
    },
  ],
});

moduleRuleTester.run(RULES.noUndeclaredEnvVars, rule, {
  valid: [
    {
      code: `
        const env2 = import.meta.env['ENV_2'];
      `,
      options: [
        { cwd: path.join(__dirname, "../../__fixtures__/workspace-configs") },
      ],
      filename: path.join(
        __dirname,
        "../../__fixtures__/workspace-configs/apps/web/index.js"
      ),
    },
    {
      code: `
        const env2 = import.meta.env["ENV_2"];
      `,
      options: [
        { cwd: path.join(__dirname, "../../__fixtures__/workspace-configs") },
      ],
      filename: path.join(
        __dirname,
        "../../__fixtures__/workspace-configs/apps/web/index.js"
      ),
    },
    {
      code: `
        const { ENV_2 } = import.meta.env;
      `,
      options: [
        { cwd: path.join(__dirname, "../../__fixtures__/workspace-configs") },
      ],
      filename: path.join(
        __dirname,
        "../../__fixtures__/workspace-configs/apps/web/index.js"
      ),
    },
    {
      code: `
        const { ROOT_DOT_ENV, WEB_DOT_ENV } = import.meta.env;
      `,
      options: [
        { cwd: path.join(__dirname, "../../__fixtures__/workspace-configs") },
      ],
      filename: path.join(
        __dirname,
        "../../__fixtures__/workspace-configs/apps/web/index.js"
      ),
    },
    {
      code: `
        const { NEXT_PUBLIC_HAHAHAHA } = import.meta.env;
      `,
      options: [
        { cwd: path.join(__dirname, "../../__fixtures__/workspace-configs") },
      ],
      filename: path.join(
        __dirname,
        "../../__fixtures__/workspace-configs/apps/web/index.js"
      ),
    },
    {
      code: `
        const { ENV_1 } = import.meta.env;
      `,
      options: [
        { cwd: path.join(__dirname, "../../__fixtures__/workspace-configs") },
      ],
      filename: path.join(
        __dirname,
        "../../__fixtures__/workspace-configs/apps/web/index.js"
      ),
    },
    {
      code: `
        const { ENV_1 } = import.meta.env;
      `,
      options: [{ cwd: "/some/random/path" }],
    },
    {
      code: `
        const { CI } = import.meta.env;
      `,
      options: [
        { cwd: path.join(__dirname, "../../__fixtures__/workspace-configs") },
      ],
      filename: path.join(
        __dirname,
        "../../__fixtures__/workspace-configs/apps/web/index.js"
      ),
    },
    {
      code: `
        const { TASK_ENV_KEY, ANOTHER_ENV_KEY } = import.meta.env;
      `,
      options: [
        { cwd: path.join(__dirname, "../../__fixtures__/configs/single") },
      ],
    },
    {
      code: `
        const { NEW_STYLE_ENV_KEY, TASK_ENV_KEY } = import.meta.env;
      `,
      options: [
        { cwd: path.join(__dirname, "../../__fixtures__/configs/single") },
      ],
    },
    {
      code: `
        const { NEW_STYLE_GLOBAL_ENV_KEY, TASK_ENV_KEY } = import.meta.env;
      `,
      options: [
        { cwd: path.join(__dirname, "../../__fixtures__/configs/single") },
      ],
    },
    {
      code: `
        const val = import.meta.env["NEW_STYLE_GLOBAL_ENV_KEY"];
      `,
      options: [
        { cwd: path.join(__dirname, "../../__fixtures__/configs/single") },
      ],
    },
    {
      code: `
        const { TASK_ENV_KEY, ANOTHER_ENV_KEY } = import.meta.env;
      `,
      options: [
        { cwd: path.join(__dirname, "../../__fixtures__/configs/single") },
      ],
    },
    {
      code: `
        const x = import.meta.env.GLOBAL_ENV_KEY;
        const { TASK_ENV_KEY, GLOBAL_ENV_KEY: renamedX } = import.meta.env;
      `,
      options: [
        { cwd: path.join(__dirname, "../../__fixtures__/configs/single") },
      ],
    },
    {
      code: "var x = import.meta.env.GLOBAL_ENV_KEY;",
      options: [
        { cwd: path.join(__dirname, "../../__fixtures__/configs/single") },
      ],
    },
    {
      code: "let x = import.meta.env.TASK_ENV_KEY;",
      options: [
        { cwd: path.join(__dirname, "../../__fixtures__/configs/single") },
      ],
    },
    {
      code: "const x = import.meta.env.ANOTHER_KEY_VALUE;",
      options: [
        {
          cwd: path.join(__dirname, "../../__fixtures__/configs/single"),
          allowList: ["^ANOTHER_KEY_[A-Z]+$"],
        },
      ],
    },
    {
      code: `
        var x = import.meta.env.ENV_VAR_ONE;
        var y = import.meta.env.ENV_VAR_TWO;
      `,
      options: [
        {
          cwd: path.join(__dirname, "../../__fixtures__/configs/single"),
          allowList: ["^ENV_VAR_[A-Z]+$"],
        },
      ],
    },
    {
      code: `
        var x = import.meta.env.ENV_VAR_ONE;
        var y = import.meta.env.ENV_VAR_TWO;
      `,
      options: [
        {
          cwd: path.join(__dirname, "../../__fixtures__/configs/single"),
          allowList: ["^ENV_VAR_O[A-Z]+$", "ENV_VAR_TWO"],
        },
      ],
    },
    {
      code: `
        var globalOrTask = import.meta.env.TASK_ENV_KEY || import.meta.env.GLOBAL_ENV_KEY;
        var oneOrTwo = import.meta.env.ENV_VAR_ONE || import.meta.env.ENV_VAR_TWO;
      `,
      options: [
        {
          cwd: path.join(__dirname, "../../__fixtures__/configs/single"),
          allowList: ["^ENV_VAR_[A-Z]+$"],
        },
      ],
    },
    {
      code: `
        () => { return import.meta.env.GLOBAL_ENV_KEY }
        () => { return import.meta.env.TASK_ENV_KEY }
        () => { return import.meta.env.ENV_VAR_ALLOWED }
      `,
      options: [
        {
          cwd: path.join(__dirname, "../../__fixtures__/configs/single"),
          allowList: ["^ENV_VAR_[A-Z]+$"],
        },
      ],
    },
    {
      code: `
        var foo = process?.env.GLOBAL_ENV_KEY
        var foo = process?.env.TASK_ENV_KEY
        var foo = process?.env.ENV_VAR_ALLOWED
      `,
      options: [
        {
          cwd: path.join(__dirname, "../../__fixtures__/configs/single"),
          allowList: ["^ENV_VAR_[A-Z]+$"],
        },
      ],
    },
    {
      code: `
        function test1(arg1 = import.meta.env.GLOBAL_ENV_KEY) {};
        function test2(arg1 = import.meta.env.TASK_ENV_KEY) {};
        function test3(arg1 = import.meta.env.ENV_VAR_ALLOWED) {};
      `,
      options: [
        {
          cwd: path.join(__dirname, "../../__fixtures__/configs/single"),
          allowList: ["^ENV_VAR_[A-Z]+$"],
        },
      ],
    },
    {
      code: `
        (arg1 = import.meta.env.GLOBAL_ENV_KEY) => {}
        (arg1 = import.meta.env.TASK_ENV_KEY) => {}
        (arg1 = import.meta.env.ENV_VAR_ALLOWED) => {}
      `,
      options: [
        {
          cwd: path.join(__dirname, "../../__fixtures__/configs/single"),
          allowList: ["^ENV_VAR_[A-Z]+$"],
        },
      ],
    },
    {
      code: "const getEnv = (key) => import.meta.env[key];",
      options: [
        { cwd: path.join(__dirname, "../../__fixtures__/configs/single") },
      ],
    },
    {
      code: "function getEnv(key) { return import.meta.env[key]; }",
      options: [
        { cwd: path.join(__dirname, "../../__fixtures__/configs/single") },
      ],
    },
    {
      code: "for (let x of ['ONE', 'TWO', 'THREE']) { console.log(import.meta.env[x]); }",
      options: [
        { cwd: path.join(__dirname, "../../__fixtures__/configs/single") },
      ],
    },
  ],

  invalid: [
    {
      code: `
        const env2 = import.meta.env['ENV_3'];
      `,
      options: [
        { cwd: path.join(__dirname, "../../__fixtures__/workspace-configs") },
      ],
      filename: path.join(
        __dirname,
        "../../__fixtures__/workspace-configs/apps/web/index.js"
      ),
      errors: [
        {
          message:
            "ENV_3 is not listed as a dependency in the root turbo.json or workspace (apps/web) turbo.json",
        },
      ],
    },
    {
      code: `
        const env2 = import.meta.env["ENV_3"];
      `,
      options: [
        { cwd: path.join(__dirname, "../../__fixtures__/workspace-configs") },
      ],
      filename: path.join(
        __dirname,
        "../../__fixtures__/workspace-configs/apps/web/index.js"
      ),
      errors: [
        {
          message:
            "ENV_3 is not listed as a dependency in the root turbo.json or workspace (apps/web) turbo.json",
        },
      ],
    },
    {
      code: `
        const { ENV_2 } = import.meta.env;
      `,
      options: [
        { cwd: path.join(__dirname, "../../__fixtures__/workspace-configs") },
      ],
      filename: path.join(
        __dirname,
        "../../__fixtures__/workspace-configs/apps/docs/index.js"
      ),
      errors: [
        {
          message:
            "ENV_2 is not listed as a dependency in the root turbo.json or workspace (apps/docs) turbo.json",
        },
      ],
    },
    {
      code: `
        const { NEXT_PUBLIC_HAHAHAHA, NEXT_PUBLIC_EXCLUDE, NEXT_PUBLIC_EXCLUDED } = import.meta.env;
      `,
      options: [
        { cwd: path.join(__dirname, "../../__fixtures__/workspace-configs") },
      ],
      filename: path.join(
        __dirname,
        "../../__fixtures__/workspace-configs/apps/web/index.js"
      ),
      errors: [
        {
          message:
            "NEXT_PUBLIC_EXCLUDE is not listed as a dependency in the root turbo.json or workspace (apps/web) turbo.json",
        },
        {
          message:
            "NEXT_PUBLIC_EXCLUDED is not listed as a dependency in the root turbo.json or workspace (apps/web) turbo.json",
        },
      ],
    },
    {
      code: "let { X } = import.meta.env;",
      options: [
        { cwd: path.join(__dirname, "../../__fixtures__/configs/single") },
      ],
      errors: [{ message: "X is not listed as a dependency in turbo.json" }],
    },
    {
      code: "const { X, Y, Z } = import.meta.env;",
      options: [
        { cwd: path.join(__dirname, "../../__fixtures__/configs/single") },
      ],
      errors: [
        { message: "X is not listed as a dependency in turbo.json" },
        { message: "Y is not listed as a dependency in turbo.json" },
        { message: "Z is not listed as a dependency in turbo.json" },
      ],
    },
    {
      code: "const { X, Y: NewName, Z } = import.meta.env;",
      options: [
        { cwd: path.join(__dirname, "../../__fixtures__/configs/single") },
      ],
      errors: [
        { message: "X is not listed as a dependency in turbo.json" },
        { message: "Y is not listed as a dependency in turbo.json" },
        { message: "Z is not listed as a dependency in turbo.json" },
      ],
    },
    {
      code: "var x = import.meta.env.NOT_THERE;",
      options: [
        { cwd: path.join(__dirname, "../../__fixtures__/configs/single") },
      ],
      errors: [
        {
          message: "NOT_THERE is not listed as a dependency in turbo.json",
        },
      ],
    },
    {
      code: "var x = import.meta.env.KEY;",
      options: [
        {
          cwd: path.join(__dirname, "../../__fixtures__/configs/single"),
          allowList: ["^ANOTHER_KEY_[A-Z]+$"],
        },
      ],
      errors: [{ message: "KEY is not listed as a dependency in turbo.json" }],
    },
    {
      code: `
        var globalOrTask = import.meta.env.TASK_ENV_KEY_NEW || import.meta.env.GLOBAL_ENV_KEY_NEW;
        var oneOrTwo = import.meta.env.ENV_VAR_ONE || import.meta.env.ENV_VAR_TWO;
      `,
      options: [
        {
          cwd: path.join(__dirname, "../../__fixtures__/configs/single"),
        },
      ],
      errors: [
        {
          message:
            "TASK_ENV_KEY_NEW is not listed as a dependency in turbo.json",
        },
        {
          message:
            "GLOBAL_ENV_KEY_NEW is not listed as a dependency in turbo.json",
        },
        {
          message: "ENV_VAR_ONE is not listed as a dependency in turbo.json",
        },
        {
          message: "ENV_VAR_TWO is not listed as a dependency in turbo.json",
        },
      ],
    },
    {
      code: `
        () => { return import.meta.env.GLOBAL_ENV_KEY_NEW }
        () => { return import.meta.env.TASK_ENV_KEY_NEW }
        () => { return import.meta.env.ENV_VAR_NOT_ALLOWED }
      `,
      options: [
        {
          cwd: path.join(__dirname, "../../__fixtures__/configs/single"),
        },
      ],
      errors: [
        {
          message:
            "GLOBAL_ENV_KEY_NEW is not listed as a dependency in turbo.json",
        },
        {
          message:
            "TASK_ENV_KEY_NEW is not listed as a dependency in turbo.json",
        },
        {
          message:
            "ENV_VAR_NOT_ALLOWED is not listed as a dependency in turbo.json",
        },
      ],
    },
    {
      code: `
        var foo = process?.env.GLOBAL_ENV_KEY_NEW
        var foo = process?.env.TASK_ENV_KEY_NEW
        var foo = process?.env.ENV_VAR_NOT_ALLOWED
      `,
      options: [
        {
          cwd: path.join(__dirname, "../../__fixtures__/configs/single"),
        },
      ],
      errors: [
        {
          message:
            "GLOBAL_ENV_KEY_NEW is not listed as a dependency in turbo.json",
        },
        {
          message:
            "TASK_ENV_KEY_NEW is not listed as a dependency in turbo.json",
        },
        {
          message:
            "ENV_VAR_NOT_ALLOWED is not listed as a dependency in turbo.json",
        },
      ],
    },
    {
      code: `
        function test1(arg1 = import.meta.env.GLOBAL_ENV_KEY_NEW) {};
        function test2(arg1 = import.meta.env.TASK_ENV_KEY_NEW) {};
        function test3(arg1 = import.meta.env.ENV_VAR_NOT_ALLOWED) {};
      `,
      options: [
        {
          cwd: path.join(__dirname, "../../__fixtures__/configs/single"),
        },
      ],
      errors: [
        {
          message:
            "GLOBAL_ENV_KEY_NEW is not listed as a dependency in turbo.json",
        },
        {
          message:
            "TASK_ENV_KEY_NEW is not listed as a dependency in turbo.json",
        },
        {
          message:
            "ENV_VAR_NOT_ALLOWED is not listed as a dependency in turbo.json",
        },
      ],
    },
    {
      code: `
        (arg1 = import.meta.env.GLOBAL_ENV_KEY_NEW) => {}
        (arg1 = import.meta.env.TASK_ENV_KEY_NEW) => {}
        (arg1 = import.meta.env.ENV_VAR_NOT_ALLOWED) => {}
      `,
      options: [
        {
          cwd: path.join(__dirname, "../../__fixtures__/configs/single"),
        },
      ],
      errors: [
        {
          message:
            "GLOBAL_ENV_KEY_NEW is not listed as a dependency in turbo.json",
        },
        {
          message:
            "TASK_ENV_KEY_NEW is not listed as a dependency in turbo.json",
        },
        {
          message:
            "ENV_VAR_NOT_ALLOWED is not listed as a dependency in turbo.json",
        },
      ],
    },
  ],
});

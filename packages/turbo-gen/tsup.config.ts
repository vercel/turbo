import { defineConfig, Options } from "tsup";
import fs from "fs-extra";
import { green } from "picocolors";
import { hex } from "@turbo/utils/src/logger";

export default defineConfig((options: Options) => ({
  entry: ["src/cli.ts", "src/types.ts"],
  format: ["cjs"],
  dts: true,
  clean: true,
  minify: true,
  onSuccess: async () => {
    // start time
    const start = Date.now();
    await fs.copy("src/templates", "dist/templates");
    // make the output match
    console.log(
      hex("#7c5cad")("TEMPLATES"),
      "copied in",
      green(`${Date.now() - start}ms`)
    );
  },
  ...options,
}));

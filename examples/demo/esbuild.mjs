import * as esbuild from "esbuild"

let cfg = {
  entryPoints: ["ts/main.ts"],
  bundle: true,
  minify: true,
  sourcemap: true,
  target: [
    "chrome58",
    "firefox57",
    "safari11",
  ],
  outfile: "static/dist/bundle.js",
};

if (process.env.WATCH) {
  let ctx = await esbuild.context(cfg);
  await ctx.watch();
} else {
  esbuild.build(cfg);
}

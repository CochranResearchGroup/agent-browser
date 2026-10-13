const fs = require('node:fs');
const path = require('node:path');
const { createRequire } = require('node:module');

async function buildWorkspaceViewportFixture(root, entry, artifacts) {
  const dashboard = path.join(root, "packages/dashboard");
  const dashboardRequire = createRequire(path.join(dashboard, "package.json"));
  const loader = path.join(artifacts, 'typescript-loader.cjs');
  fs.writeFileSync(loader, `const ts=require(${JSON.stringify(dashboardRequire.resolve('typescript'))});
module.exports=function(source){return ts.transpileModule(source,{fileName:this.resourcePath,compilerOptions:{module:ts.ModuleKind.ESNext,target:ts.ScriptTarget.ES2020,jsx:ts.JsxEmit.ReactJSX,esModuleInterop:true}}).outputText;};`);
  const { webpack } = dashboardRequire('next/dist/compiled/webpack/webpack');
  await new Promise((resolve, reject) => webpack({
    mode: 'development', context: dashboard,
    entry,
    output: { path: artifacts, filename: 'bundle.js' }, devtool: false,
    resolve: {
      extensions: ['.tsx', '.ts', '.jsx', '.js', '.json'],
      alias: { '@': path.join(dashboard, 'src') },
      modules: [path.join(dashboard, 'node_modules'), path.join(root, 'node_modules'), 'node_modules'],
    },
    module: { rules: [{ test: /\.tsx?$/, use: [loader] }] },
  }, (error, stats) => {
    if (error) return reject(error);
    if (stats.hasErrors()) return reject(new Error(stats.toString({ all: false, errors: true, colors: false })));
    resolve();
  }));
}

module.exports = { buildWorkspaceViewportFixture };

# Contributing

If you want to modify the way that the highlighting of Cairo code sample works, you need to:

1. Clone the repository at https://github.com/highlightjs/highlight.js
2. Copy our [cairo.js](./cairo.js) into `highlight.js/src/languages` directory
3. Run `npm install`
4. Modify the copied `cairo.js`
5. Run `node tools/build.js :common apache armasm coffeescript d handlebars haskell http julia nginx nim nix properties r scala x86asm yaml cairo`
6. Replace our [highlight.js](./highlight.js) content with newly built `highlight.js/build/highlight.min.js`

const rust = import('./dist/druid_web_example');

rust
  .then(m => m.run())
  .catch(console.error);

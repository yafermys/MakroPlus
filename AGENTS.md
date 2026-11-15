# Repository instructions

## Publishing releases

Whenever publishing a new GitHub release, update the English GitHub landing page
(`README.md`) in the same task, without waiting for a separate request. Describe
new capabilities in simple terms, refresh the download/source/release-note links,
and update relevant screenshots when the interface changes. Keep the page short;
put detailed coverage, setup and limitations in the guide and linked docs.

Also update the changelog, matching release notes, offline instructions and
application/package versions. Verify the build, appropriate tests, package hashes,
source archive and GitHub checks before marking a release as the latest stable.

Never publish local settings, API keys, actual machine-scan reports, personal
screenshots or language-model weights. Use synthetic data for public screenshots.
Keep the documented models folder in distributions without bundling a model.

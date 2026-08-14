// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

import assert from "node:assert/strict";
import { mkdtemp, realpath, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

import {
  DefaultPackageManager,
  DefaultResourceLoader,
  SettingsManager,
} from "@earendil-works/pi-coding-agent";

const extensionRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const repositoryRoot = resolve(extensionRoot, "../..");
const expectedExtension = await realpath(join(extensionRoot, "index.ts"));
const expectedSkill = await realpath(
  join(extensionRoot, "skills/smth/SKILL.md"),
);
const agentDir = await mkdtemp(join(tmpdir(), "pi-smth-smoke-"));

try {
  const packageManager = new DefaultPackageManager({
    cwd: agentDir,
    agentDir,
    settingsManager: SettingsManager.inMemory(),
  });

  for (const packageRoot of [extensionRoot, repositoryRoot]) {
    const resources = await packageManager.resolveExtensionSources(
      [packageRoot],
      { temporary: true },
    );
    const extensions = await Promise.all(
      resources.extensions
        .filter((resource) => resource.enabled)
        .map((resource) => realpath(resource.path)),
    );
    const skills = await Promise.all(
      resources.skills
        .filter((resource) => resource.enabled)
        .map((resource) => realpath(resource.path)),
    );

    assert(
      extensions.includes(expectedExtension),
      `${packageRoot} did not discover the pi-smth extension`,
    );
    assert(
      skills.includes(expectedSkill),
      `${packageRoot} did not discover the smth skill`,
    );

    const loader = new DefaultResourceLoader({
      cwd: agentDir,
      agentDir,
      additionalSkillPaths: [expectedSkill],
      noContextFiles: true,
      noExtensions: true,
      noPromptTemplates: true,
      noThemes: true,
    });
    await loader.reload();

    const { skills: loaded, diagnostics } = loader.getSkills();
    assert.equal(
      diagnostics.length,
      0,
      `invalid smth skill: ${JSON.stringify(diagnostics)}`,
    );
    assert(
      loaded.some(
        (skill) =>
          skill.name === "smth" &&
          resolve(skill.filePath) === resolve(expectedSkill),
      ),
      `${packageRoot} did not load the smth skill`,
    );
  }

  const packed = spawnSync(
    "npm",
    ["pack", "--dry-run", "--json", "--ignore-scripts"],
    {
      cwd: extensionRoot,
      encoding: "utf8",
      env: { ...process.env, npm_config_cache: join(agentDir, "npm-cache") },
    },
  );
  assert.equal(packed.status, 0, packed.stderr || packed.stdout);

  const [{ files }] = JSON.parse(packed.stdout);
  assert(
    files.some((file) => file.path === "skills/smth/SKILL.md"),
    "published pi-smth package omitted the smth skill",
  );

  console.log("pi-smth extension and skill discovered from both package roots");
} finally {
  await rm(agentDir, { force: true, recursive: true });
}

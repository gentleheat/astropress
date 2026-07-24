import type { AstropressAppHost } from "./app-host-targets";
import type { AstropressDataServices } from "./data-service-targets";
import type { AstropressDonationsProviders } from "./project-scaffold";
import {
	createAstropressConfig,
	createAstropressPublicConfig,
	createDonatePage,
	createQualityWorkflow,
	createSecurityWorkflow,
	gitHubActionsDeployWorkflow,
	gitLabPagesWorkflow,
	isStaticOnlyHost,
} from "./project-scaffold-ci-helpers";
import { appHostToDeployTarget } from "./project-scaffold-env";

export function createPackageScripts(appHost: AstropressAppHost) {
	const scripts: Record<string, string> = {
		dev: "astro dev",
		build: "astro build",
		check: "astro check",
		test: "vitest run --passWithNoTests",
		lint: "bunx biome check src",
		format: "bunx biome format --write src",
		"doctor:strict": "astropress doctor --strict",
		// `lefthook install` exits non-zero outside a work tree (and when git is
		// not installed at all), which would surface as a failed `prepare` on the
		// user's very first `bun install`. `astropress new` initialises a repo so
		// the hooks do get installed in the normal case; this keeps the rarer
		// no-git case quiet instead of looking like a broken scaffold.
		prepare: "bunx lefthook install || true",
	};

	// Server-output hosts ship the admin surface. For the two-site topology
	// (admin/test + public/prod) we also expose a static-only build that uses
	// the secondary public config. See docs/guides/TWO_SITE_DEPLOY.md.
	if (!isStaticOnlyHost(appHost)) {
		scripts["build:public"] = "astro build --config astro.config.public.mjs";
	}

	switch (appHost) {
		case "cloudflare-pages":
			scripts["deploy:cloudflare"] = "wrangler pages deploy dist --commit-dirty=true";
			scripts["build:cloudflare-production"] = "astro build";
			break;
		case "vercel":
			scripts["deploy:vercel"] = "vercel build && vercel deploy --prebuilt --prod --yes";
			break;
		case "netlify":
			scripts["deploy:netlify"] = "netlify deploy --dir dist --prod";
			break;
		case "render-static":
			scripts["deploy:render-static"] = "astro build";
			break;
		case "render-web":
			scripts["deploy:render-web"] = "astro build";
			break;
		case "gitlab-pages":
			scripts["deploy:gitlab-pages"] = "astro build";
			break;
		case "fly-io":
			scripts["deploy:fly-io"] = "flyctl deploy --remote-only";
			break;
		case "railway":
			scripts["deploy:railway"] = "railway up";
			break;
		case "digitalocean":
			scripts["deploy:digitalocean"] = "doctl apps create-deployment $DO_APP_ID";
			break;
		case "coolify":
			// Coolify deploys via git push / webhooks — no CLI deploy step needed.
			scripts["deploy:coolify"] = "astro build";
			break;
		case "custom":
			scripts["deploy:custom"] = "astro build";
			break;
		default:
			break;
	}

	return scripts;
}

export function createCiFiles(
	appHost: AstropressAppHost,
	requiredEnvKeys: string[],
	donations?: AstropressDonationsProviders,
) {
	const files: Record<string, string> = {};
	if (appHost === "gitlab-pages") {
		files[".gitlab-ci.yml"] = gitLabPagesWorkflow();
	} else {
		files[".github/workflows/deploy-astropress.yml"] = gitHubActionsDeployWorkflow(
			appHost,
			requiredEnvKeys,
		);
		files[".github/workflows/quality.yml"] = createQualityWorkflow();
		files[".github/workflows/security.yml"] = createSecurityWorkflow();
	}
	files["astro.config.mjs"] = createAstropressConfig(appHost);
	// Emit a second public-site config so server-output projects can build a
	// zero-admin static bundle for their production origin — this is the
	// recommended two-site deployment default.
	if (!isStaticOnlyHost(appHost)) {
		files["astro.config.public.mjs"] = createAstropressPublicConfig();
	}
	const hasAnyDonation =
		donations && (donations.giveLively || donations.liberapay || donations.pledgeCrypto);
	if (hasAnyDonation) {
		files["src/pages/donate.astro"] = createDonatePage(donations, "https://example.com");
	}
	return files;
}

export function createDeployDoc(
	appHost: AstropressAppHost,
	dataServices: AstropressDataServices,
	supportLevel: string,
	requiredEnvKeys: string[],
) {
	const deployTarget = appHostToDeployTarget(appHost);
	const envList =
		requiredEnvKeys.length === 0
			? "- No extra Content Services secrets are required.\n"
			: `${requiredEnvKeys.map((key) => `- \`${key}\``).join("\n")}\n`;
	const serviceOriginNote =
		dataServices === "none"
			? ""
			: `- Set \`ASTROPRESS_SERVICE_ORIGIN\` to the Astropress service endpoint for your ${dataServices} setup.\n`;

	const isStatic = appHost === "github-pages" || appHost === "gitlab-pages";
	const twoSiteBlock = isStatic
		? ""
		: `
## Two-site deployment (admin + public)

This scaffold emits **two** Astro configs because Astropress is designed to run as a pair
of sites on separate origins:

| File | Output | Role | Where it deploys |
|------|--------|------|------------------|
| \`astro.config.mjs\` | \`server\` | Admin / test environment (editors sign in at \`/ap-admin\`) | \`${deployTarget}\` (this app host) |
| \`astro.config.public.mjs\` | \`static\` | Public production site (zero admin surface) | A static host such as GitHub Pages, GitLab Pages, or a CDN |

The split is enforced by \`createAstropressPublicSiteIntegration\`, which injects zero
\`/ap-admin\` routes and no security middleware — so the static bundle literally cannot
serve admin code even if the CDN is misrouted.

### Build commands

\`\`\`bash
bun run build           # admin/test bundle (what this app host deploys)
bun run build:public    # public/prod static bundle (ship dist/ to the prod origin)
\`\`\`

### Choosing a topology

- **One repo, two configs (default):** simplest to get running. Both builds live
  in the same repository and the CI pipeline can invoke each one independently.
- **Two repos:** stronger isolation and better for teams that split admin/editorial
  ownership from site engineering. See \`docs/guides/TWO_SITE_DEPLOY.md\` for the
  full walkthrough including a \`repository_dispatch\` webhook recipe.

Either way, the admin host and the public host should run on **different origins**
(e.g. \`admin.example.com\` vs \`example.com\`) so cookies never cross boundaries.
`;

	return `# Deploy Astropress

This project is scaffolded for:

- App Host: \`${appHost}\`
- Content Services: \`${dataServices}\`
- Support level: \`${supportLevel}\`
- Deploy target: \`${deployTarget}\`

## Local checks

Run these before pushing:

\`\`\`bash
bun install
bun run doctor:strict
bun run build
\`\`\`

## Required secrets and variables

${envList}${serviceOriginNote}## CI

- A deploy workflow was generated for the selected app host.
- The workflow always installs dependencies, runs \`astropress doctor --strict\`, and builds before publishing.
- For Render, deployment is automatic only if \`RENDER_DEPLOY_HOOK_URL\` is configured.

## Scope

- The App Host only publishes the Astro web app and admin shell.
- Content Services hold content, media, sessions, and the Astropress service API.
- If Content Services are not \`none\`, deployment is incomplete until those service credentials and \`ASTROPRESS_SERVICE_ORIGIN\` are configured.
${twoSiteBlock}`;
}

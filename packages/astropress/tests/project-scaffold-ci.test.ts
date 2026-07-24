import { describe, expect, it } from "vitest";
import type { AstropressAppHost } from "../src/app-host-targets";
import { createCiFiles, createDeployDoc, createPackageScripts } from "../src/project-scaffold-ci";

describe("createPackageScripts — base scripts present for every host", () => {
	const cases: AstropressAppHost[] = [
		"cloudflare-pages",
		"vercel",
		"netlify",
		"render-static",
		"render-web",
		"github-pages",
		"gitlab-pages",
		"fly-io",
		"railway",
		"digitalocean",
		"coolify",
		"custom",
	];
	for (const host of cases) {
		it(`pins the base script map for ${host}`, () => {
			const scripts = createPackageScripts(host);
			expect(scripts.dev).toBe("astro dev");
			expect(scripts.build).toBe("astro build");
			expect(scripts.check).toBe("astro check");
			expect(scripts.test).toBe("vitest run --passWithNoTests");
			expect(scripts.lint).toBe("bunx biome check src");
			expect(scripts.format).toBe("bunx biome format --write src");
			expect(scripts["doctor:strict"]).toBe("astropress doctor --strict");
			// Trailing `|| true`: `lefthook install` exits non-zero outside a git
			// work tree, and a failed `prepare` would make the user's first
			// `bun install` after `astropress new` look like a broken scaffold.
			expect(scripts.prepare).toBe("bunx lefthook install || true");
		});
	}
});

describe("createPackageScripts — static-only hosts skip build:public", () => {
	it("omits build:public for github-pages and gitlab-pages", () => {
		expect(createPackageScripts("github-pages")["build:public"]).toBeUndefined();
		expect(createPackageScripts("gitlab-pages")["build:public"]).toBeUndefined();
	});
	it("includes build:public for server-output hosts", () => {
		const serverHosts: AstropressAppHost[] = [
			"cloudflare-pages",
			"vercel",
			"netlify",
			"render-web",
			"fly-io",
			"railway",
			"digitalocean",
			"coolify",
			"custom",
		];
		for (const h of serverHosts) {
			expect(createPackageScripts(h)["build:public"]).toBe(
				"astro build --config astro.config.public.mjs",
			);
		}
	});
});

describe("createPackageScripts — per-host deploy commands (pin exact strings)", () => {
	it("cloudflare-pages emits wrangler pages deploy + cloudflare-production build", () => {
		const s = createPackageScripts("cloudflare-pages");
		expect(s["deploy:cloudflare"]).toBe("wrangler pages deploy dist --commit-dirty=true");
		expect(s["build:cloudflare-production"]).toBe("astro build");
	});
	it("vercel emits vercel build && vercel deploy --prebuilt --prod --yes", () => {
		expect(createPackageScripts("vercel")["deploy:vercel"]).toBe(
			"vercel build && vercel deploy --prebuilt --prod --yes",
		);
	});
	it("netlify emits netlify deploy --dir dist --prod", () => {
		expect(createPackageScripts("netlify")["deploy:netlify"]).toBe(
			"netlify deploy --dir dist --prod",
		);
	});
	it("render-static emits astro build as deploy", () => {
		expect(createPackageScripts("render-static")["deploy:render-static"]).toBe("astro build");
	});
	it("render-web emits astro build as deploy", () => {
		expect(createPackageScripts("render-web")["deploy:render-web"]).toBe("astro build");
	});
	it("gitlab-pages emits astro build as deploy", () => {
		expect(createPackageScripts("gitlab-pages")["deploy:gitlab-pages"]).toBe("astro build");
	});
	it("fly-io emits flyctl deploy --remote-only", () => {
		expect(createPackageScripts("fly-io")["deploy:fly-io"]).toBe("flyctl deploy --remote-only");
	});
	it("railway emits railway up", () => {
		expect(createPackageScripts("railway")["deploy:railway"]).toBe("railway up");
	});
	it("digitalocean emits doctl apps create-deployment $DO_APP_ID", () => {
		expect(createPackageScripts("digitalocean")["deploy:digitalocean"]).toBe(
			"doctl apps create-deployment $DO_APP_ID",
		);
	});
	it("coolify emits astro build as deploy", () => {
		expect(createPackageScripts("coolify")["deploy:coolify"]).toBe("astro build");
	});
	it("custom emits astro build as deploy", () => {
		expect(createPackageScripts("custom")["deploy:custom"]).toBe("astro build");
	});
	it("github-pages does not emit a deploy:<host> script", () => {
		const s = createPackageScripts("github-pages");
		const deployKeys = Object.keys(s).filter((k) => k.startsWith("deploy:"));
		expect(deployKeys).toEqual([]);
	});
});

describe("createCiFiles — file set per host", () => {
	it("gitlab-pages emits .gitlab-ci.yml and no github workflows", () => {
		const files = createCiFiles("gitlab-pages", []);
		expect(files[".gitlab-ci.yml"]).toBeTruthy();
		expect(files[".github/workflows/deploy-astropress.yml"]).toBeUndefined();
		expect(files[".github/workflows/quality.yml"]).toBeUndefined();
		expect(files[".github/workflows/security.yml"]).toBeUndefined();
	});

	it("non-gitlab hosts emit deploy, quality, and security workflows but no .gitlab-ci.yml", () => {
		const files = createCiFiles("cloudflare-pages", ["CLOUDFLARE_API_TOKEN"]);
		expect(files[".github/workflows/deploy-astropress.yml"]).toBeTruthy();
		expect(files[".github/workflows/quality.yml"]).toBeTruthy();
		expect(files[".github/workflows/security.yml"]).toBeTruthy();
		expect(files[".gitlab-ci.yml"]).toBeUndefined();
	});

	it("every host emits astro.config.mjs", () => {
		expect(createCiFiles("cloudflare-pages", [])["astro.config.mjs"]).toBeTruthy();
		expect(createCiFiles("gitlab-pages", [])["astro.config.mjs"]).toBeTruthy();
		expect(createCiFiles("github-pages", [])["astro.config.mjs"]).toBeTruthy();
	});

	it("static-only hosts skip astro.config.public.mjs; server-output hosts emit it", () => {
		expect(createCiFiles("github-pages", [])["astro.config.public.mjs"]).toBeUndefined();
		expect(createCiFiles("gitlab-pages", [])["astro.config.public.mjs"]).toBeUndefined();
		expect(createCiFiles("cloudflare-pages", [])["astro.config.public.mjs"]).toBeTruthy();
		expect(createCiFiles("vercel", [])["astro.config.public.mjs"]).toBeTruthy();
	});

	it("emits donate page when any donations.* is truthy", () => {
		expect(
			createCiFiles("vercel", [], { giveLively: true })["src/pages/donate.astro"],
		).toBeTruthy();
		expect(
			createCiFiles("vercel", [], { liberapay: "user" })["src/pages/donate.astro"],
		).toBeTruthy();
		expect(
			createCiFiles("vercel", [], { pledgeCrypto: { btc: "addr" } })["src/pages/donate.astro"],
		).toBeTruthy();
	});

	it("omits donate page when donations is undefined or all-false", () => {
		expect(createCiFiles("vercel", [])["src/pages/donate.astro"]).toBeUndefined();
		expect(createCiFiles("vercel", [], {})["src/pages/donate.astro"]).toBeUndefined();
	});
});

describe("createDeployDoc — content and conditional branches", () => {
	it("includes app-host, data-services, support-level, and deploy-target lines", () => {
		const doc = createDeployDoc("cloudflare-pages", "cloudflare", "community", [
			"CLOUDFLARE_API_TOKEN",
		]);
		expect(doc).toContain("- App Host: `cloudflare-pages`");
		expect(doc).toContain("- Content Services: `cloudflare`");
		expect(doc).toContain("- Support level: `community`");
		expect(doc).toContain("- Deploy target: `cloudflare`");
	});

	it("emits 'No extra Content Services secrets' line when requiredEnvKeys is empty", () => {
		const doc = createDeployDoc("vercel", "none", "community", []);
		expect(doc).toContain("- No extra Content Services secrets are required.");
	});

	it("renders each requiredEnvKey as a backticked bullet when non-empty", () => {
		const doc = createDeployDoc("vercel", "supabase", "community", ["A_KEY", "B_KEY"]);
		expect(doc).toContain("- `A_KEY`");
		expect(doc).toContain("- `B_KEY`");
		expect(doc).not.toContain("No extra Content Services secrets");
	});

	it("emits ASTROPRESS_SERVICE_ORIGIN note when dataServices !== 'none'", () => {
		const doc = createDeployDoc("vercel", "supabase", "community", []);
		expect(doc).toContain(
			"Set `ASTROPRESS_SERVICE_ORIGIN` to the Astropress service endpoint for your supabase setup.",
		);
	});

	it("omits the per-service ASTROPRESS_SERVICE_ORIGIN bullet when dataServices === 'none'", () => {
		const doc = createDeployDoc("vercel", "none", "community", []);
		expect(doc).not.toContain("Set `ASTROPRESS_SERVICE_ORIGIN` to the Astropress");
	});

	it("omits the two-site block for github-pages (static host)", () => {
		const doc = createDeployDoc("github-pages", "none", "community", []);
		expect(doc).not.toContain("## Two-site deployment");
	});

	it("omits the two-site block for gitlab-pages (static host)", () => {
		const doc = createDeployDoc("gitlab-pages", "none", "community", []);
		expect(doc).not.toContain("## Two-site deployment");
	});

	it("includes the two-site block for server-output hosts (e.g. cloudflare-pages)", () => {
		const doc = createDeployDoc("cloudflare-pages", "cloudflare", "community", []);
		expect(doc).toContain("## Two-site deployment (admin + public)");
		expect(doc).toContain("`astro.config.mjs`");
		expect(doc).toContain("`astro.config.public.mjs`");
		expect(doc).toContain("createAstropressPublicSiteIntegration");
		expect(doc).toContain("bun run build:public");
		expect(doc).toContain("docs/guides/TWO_SITE_DEPLOY.md");
	});

	it("two-site table references the deploy target (cloudflare, not cloudflare-pages)", () => {
		const doc = createDeployDoc("cloudflare-pages", "cloudflare", "community", []);
		expect(doc).toContain("| `cloudflare` (this app host) |");
	});

	it("two-site table references the deploy target for non-cloudflare hosts (vercel)", () => {
		const doc = createDeployDoc("vercel", "supabase", "community", []);
		expect(doc).toContain("| `vercel` (this app host) |");
	});

	it("emits the Local checks code block and CI section headers", () => {
		const doc = createDeployDoc("vercel", "supabase", "community", []);
		expect(doc).toContain("## Local checks");
		expect(doc).toContain("bun install");
		expect(doc).toContain("bun run doctor:strict");
		expect(doc).toContain("bun run build");
		expect(doc).toContain("## Required secrets and variables");
		expect(doc).toContain("## CI");
		expect(doc).toContain("## Scope");
		expect(doc).toContain("RENDER_DEPLOY_HOOK_URL");
	});

	it("starts with the # Deploy Astropress heading", () => {
		const doc = createDeployDoc("vercel", "none", "community", []);
		expect(doc.startsWith("# Deploy Astropress\n")).toBe(true);
	});
});

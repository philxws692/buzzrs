// @ts-check
import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";
import starlightLinksValidator from "starlight-links-validator";

// https://astro.build/config
export default defineConfig({
  site: "https://buzzrs.philslr.de",
  integrations: [
    starlight({
      plugins: [starlightLinksValidator()],
      title: "buzzrs",
      favicon: "/favicon.png",
      social: [
        {
          icon: "github",
          label: "GitHub",
          href: "https://github.com/philxws692/buzzrs",
        },
      ],
      sidebar: [
        {
          label: "Guide",
          items: [
            // Each item here is one entry in the navigation menu.
            { autogenerate: { directory: "guides" } },
          ],
        },
        {
          label: "Services",
          items: [
            { label: "Services Overview", slug: "services/overview" },
            { label: "Discord", slug: "services/discord" },
            { label: "Gotify", slug: "services/gotify" },
            { label: "Ntfy.sh", slug: "services/ntfy" },
          ],
        },
      ],
    }),
  ],
});

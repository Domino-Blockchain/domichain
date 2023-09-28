const math = require("remark-math");
const katex = require("rehype-katex");
module.exports = {
  title: "Domichain Docs",
  tagline:
    "Domichain is an open source project implementing a new, high-performance, permissionless blockchain.",
  url: "https://docs.domichain.com",
  baseUrl: "/",
  favicon: "img/favicon.ico",
  organizationName: "domichain-labs", // Usually your GitHub org/user name.
  projectName: "domichain", // Usually your repo name.
  onBrokenLinks: "throw",
  stylesheets: [
    {
      href: "/katex/katex.min.css",
      type: "text/css",
      integrity:
        "sha384-AfEj0r4/OFrOo5t7NnNe46zW/tFgW6x/bCJG8FqQCEo3+Aro6EYUG4+cU+KJWu/X",
      crossorigin: "anonymous",
    },
  ],
  i18n: {
    defaultLocale: "en",
    locales: ["en", "de", "es", "ru", "ar"],
    // localesNotBuilding: ["ko", "pt", "vi", "zh", "ja"],
    localeConfigs: {
      en: {
        label: "English",
      },
      ru: {
        label: "Русский",
      },
      es: {
        label: "Español",
      },
      de: {
        label: "Deutsch",
      },
      ar: {
        label: "العربية",
      },
      ko: {
        label: "한국어",
      },
    },
  },
  themeConfig: {
    prism: {
      additionalLanguages: ["rust"],
    },
    navbar: {
      logo: {
        alt: "Domichain Logo",
        src: "img/logo-horizontal.svg",
        srcDark: "img/logo-horizontal-dark.svg",
      },
      items: [
        {
          to: "introduction",
          label: "Learn",
          position: "left",
        },
        {
          to: "cluster/overview",
          label: "Architecture",
          position: "left",
        },
        {
          to: "cli",
          label: "CLI",
          position: "left",
        },
        {
          to: "/developers",
          label: "Developers",
          position: "left",
        },
        {
          to: "running-validator",
          label: "Validators",
          position: "left",
        },
        {
          label: "More",
          position: "left",
          items: [
            { label: "Terminology", to: "terminology" },
            { label: "Staking", to: "staking" },
            { label: "Integrations", to: "integrations/exchange" },
            { label: "Economics", to: "economics_overview" },
            { label: "Proposals", to: "proposals" },
            {
              href: "https://spl.domichain.com",
              label: "Domichain Program Library »",
            },
          ],
        },
        {
          type: "localeDropdown",
          position: "right",
        },
        {
          href: "https://domichain.com/discord",
          // label: "Discord",
          className: "header-link-icon header-discord-link",
          "aria-label": "Domichain Discord",
          position: "right",
        },
        {
          href: "https://Domino-Blockchain/domichain",
          // label: "GitHub",
          className: "header-link-icon header-github-link",
          "aria-label": "GitHub repository",
          position: "right",
        },
      ],
    },
    algolia: {
      // This API key is "search-only" and safe to be published
      apiKey: "011e01358301f5023b02da5db6af7f4d",
      appId: "FQ12ISJR4B",
      indexName: "domichain",
      contextualSearch: true,
    },
    footer: {
      style: "dark",
      links: [
        {
          title: "Documentation",
          items: [
            {
              label: "Learn",
              to: "introduction",
            },
            {
              label: "Developers",
              to: "/developers",
            },
            {
              label: "Validators",
              to: "running-validator",
            },
            {
              label: "Command Line",
              to: "cli",
            },
            {
              label: "Architecture",
              to: "cluster/overview",
            },
          ],
        },
        {
          title: "Community",
          items: [
            {
              label: "Stack Exchange »",
              href: "https://domichain.stackexchange.com/",
            },
            {
              label: "GitHub »",
              href: "https://Domino-Blockchain/domichain",
            },
            {
              label: "Discord »",
              href: "https://domichain.com/discord",
            },
            {
              label: "Twitter »",
              href: "https://twitter.com/domichain",
            },
            {
              label: "Forum »",
              href: "https://forum.domichain.com",
            },
          ],
        },
        {
          title: "Resources",
          items: [
            {
              label: "Proposals",
              to: "proposals",
            },
            {
              label: "Integrations",
              to: "integrations/exchange",
            },
            {
              href: "https://spl.domichain.com",
              label: "Domichain Program Library »",
            },
          ],
        },
      ],
      copyright: `Copyright © ${new Date().getFullYear()} Domichain Foundation`,
    },
  },
  presets: [
    [
      "@docusaurus/preset-classic",
      {
        docs: {
          path: "src",
          breadcrumbs: false,
          routeBasePath: "/",
          sidebarPath: require.resolve("./sidebars.js"),
          remarkPlugins: [math],
          rehypePlugins: [katex],
        },
        theme: {
          customCss: require.resolve("./src/css/custom.css"),
        },
        // Google Analytics are only active in prod
        gtag: {
          // this GA code is safe to be published
          trackingID: "G-94WS0LRZRS",
          anonymizeIP: true,
        },
      },
    ],
  ],
};

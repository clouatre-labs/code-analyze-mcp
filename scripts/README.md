# fill-openssf-badge

Playwright TypeScript script that fills the OpenSSF Best Practices badge form
for project 12275 (code-analyze-mcp).

## Prerequisites

- [bun](https://bun.sh) 1.3.x or later
- Playwright Chromium installed:

  ```
  cd scripts
  bun install
  bunx playwright install chromium
  ```

- A GitHub account that has edit access to
  https://www.bestpractices.dev/en/projects/12275

## How to run

```
cd scripts
bun install
bun run fill-openssf-badge.ts
```

## What it does

1. Launches a headed Chromium browser (not headless -- you can see it).
2. Navigates to the passing-level edit form at
   https://www.bestpractices.dev/en/projects/12275/passing/edit.
3. If the site redirects to /en/login, it pauses and prints:

   ```
   Please complete GitHub OAuth login in the browser, then press Enter to continue...
   ```

   Complete the GitHub OAuth flow in the browser window, then press Enter in
   the terminal.

4. After login, fills all passing-level criteria (radio buttons and
   justification text areas) with pre-assessed answers from the research phase.
5. Clicks "Submit and Exit" to save the passing section.
6. Navigates to the silver-level edit form and fills all silver-level criteria.
7. Clicks "Submit and Exit" to save the silver section.
8. Prints "Done." and closes the browser.

## Notes

- The script uses a 80 ms delay between each criterion to avoid bot detection.
  The full run takes a few minutes.
- `achieve_passing_status` and `achieve_silver_status` are computed server-side
  and are not set by the script.
- Silver criteria that are not yet assessed in the documentation
  (crypto_tls12, regression_tests_added50, test_statement_coverage80,
  hardened_site) are skipped and left at their current value on the site.
- If the script fails mid-run, re-running it is safe -- fields are overwritten
  idempotently.
- The `vulnerabilities_fixed_60_days` criterion (not present in the main
  OpenSSF criteria doc but present in the project JSON) is filled with the
  same Met answer as `vulnerabilities_critical_fixed`.

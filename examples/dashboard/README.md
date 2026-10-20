# Yav SaaS Dashboard Example

This is the official production demonstration application for the **Yav.rs** web framework, fulfilling Section 15 of the master specification (`prompt.md`).

## Features Demonstrated

- **SaaS Layout**: Responsive sidebar navigation with active badges, status indicators, and sticky navbar.
- **Dark & Light Mode**: Seamless theme switching with anti-FOUC script and `localStorage` persistence.
- **KPI Stat Cards**: Revenue, tenant counts, edge latency, and memory utilization with trend indicators.
- **Live Visual SVG Charts**: Custom inline SVG charts without external charting libraries or heavy JS bundles.
- **Recent Activity Table**: Multi-column data table with user avatars, status pills, and monetary amounts.
- **Component Showroom (`/components`)**: Interactive gallery showcasing 40+ primitives from `yav-ui` (Buttons, Badges, Alerts, Accordions, Steppers, and Client Islands).
- **Searchable Icon Explorer (`/icons`)**: Instant live keyword search over 80+ embedded Lucide SVG icons with one-click code copy.
- **Dynamic Routing & Data Loading (`/projects` & `/projects/:id`)**: Server-side data loading via `Loader<T>` and automatic URL parameter extraction via `RouteContext.param("id")`.
- **Form Actions & Security (`/settings`)**: Mutations with CSRF validation, input validation, and alert notification feedback.

## Running the Dashboard

```bash
# Using the Yav CLI
yav dev

# Or directly using Cargo
cargo run
```

Then visit [http://127.0.0.1:3000](http://127.0.0.1:3000) in your browser!

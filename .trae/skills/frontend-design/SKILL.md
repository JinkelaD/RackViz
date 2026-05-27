---
name: "frontend-design"
description: "Provides frontend UI/UX design expertise including color theory, typography, responsive layout, component design patterns, CSS styling best practices, and accessibility guidelines. Invoke when user asks for frontend design advice, UI improvements, layout optimization, styling suggestions, or component redesign."
---

# Frontend Design

This skill provides comprehensive frontend design expertise for building modern, professional, and user-friendly web interfaces.

## Design Principles

### 1. Visual Hierarchy
- Use size, color, contrast, and spacing to guide user attention
- Most important elements should be most prominent
- Maintain clear information architecture with consistent heading levels
- Use whitespace strategically to separate content sections

### 2. Color Theory
- **Primary Color**: Main brand color, used for key interactive elements
- **Secondary Color**: Complementary accent color
- **Neutral Colors**: Grays for text, backgrounds, borders
- **Semantic Colors**: Red (error/danger), Green (success), Yellow (warning), Blue (info)
- Maintain WCAG 2.1 AA contrast ratios (4.5:1 for normal text, 3:1 for large text)
- Use CSS custom properties (variables) for consistent theming

### 3. Typography
- Use system font stacks for optimal rendering across platforms
- Limit to 2-3 font families per project
- Establish clear type scale (e.g., 12px, 14px, 16px, 20px, 24px, 32px)
- Line height: 1.5 for body text, 1.2 for headings
- Font weight: 400 (regular), 500 (medium), 600 (semibold), 700 (bold)

### 4. Spacing System
- Use a consistent spacing scale (4px/8px base):
  - xs: 4px, sm: 8px, md: 16px, lg: 24px, xl: 32px, 2xl: 48px
- Apply consistent padding within components
- Maintain visual rhythm with equal or proportional margins

### 5. Responsive Design
- Mobile-first approach: design for smallest screens first
- Common breakpoints:
  - Mobile: 320px - 768px
  - Tablet: 768px - 1024px
  - Desktop: 1024px - 1440px
  - Wide: 1440px+
- Use relative units (rem, %, vw/vh) over fixed pixels
- Test layouts at all breakpoints

## Component Design Patterns

### Buttons
- **Primary**: Solid background, high contrast (main CTA)
- **Secondary**: Outlined style (alternative actions)
- **Ghost/Text**: No background, minimal visual weight
- **Danger**: Red tones for destructive actions
- Include hover, active, disabled, and focus states
- Minimum touch target: 44x44px (WCAG)

### Cards
- Container with subtle shadow and border-radius
- Clear content sections: header, body, footer
- Consistent padding (16-24px)
- Hover states for interactive cards

### Forms
- Labels above or to the left of inputs
- Clear validation states (error, success)
- Helpful placeholder text
- Appropriate input types and sizes
- Focus indicators for accessibility

### Navigation
- Clear current page/active state indicators
- Responsive: hamburger menu on mobile, full nav on desktop
- Breadcrumbs for deep hierarchies
- Sticky headers for long-scrolling pages

### Data Display
- Tables with alternating row colors for readability
- Pagination for large datasets
- Empty states with helpful messaging
- Loading skeletons instead of spinners where possible

### Modals & Dialogs
- Overlay backdrop with blur/dim effect
- Focus trap within modal
- Escape key to close
- Responsive sizing (full-screen on mobile)

## CSS Best Practices

### CSS Variables (Custom Properties)
```css
:root {
  --color-primary: #1890ff;
  --color-primary-hover: #40a9ff;
  --color-primary-active: #096dd9;
  --color-success: #52c41a;
  --color-warning: #faad14;
  --color-error: #ff4d4f;
  --color-bg: #ffffff;
  --color-bg-secondary: #f5f5f5;
  --color-text: #262626;
  --color-text-secondary: #8c8c8c;
  --color-border: #d9d9d9;
  --radius-sm: 4px;
  --radius-md: 8px;
  --radius-lg: 12px;
  --shadow-sm: 0 1px 2px rgba(0,0,0,0.06);
  --shadow-md: 0 4px 12px rgba(0,0,0,0.08);
  --shadow-lg: 0 8px 24px rgba(0,0,0,0.12);
  --space-xs: 4px;
  --space-sm: 8px;
  --space-md: 16px;
  --space-lg: 24px;
  --space-xl: 32px;
  --font-size-sm: 12px;
  --font-size-md: 14px;
  --font-size-lg: 16px;
  --font-size-xl: 20px;
  --font-size-2xl: 24px;
}
```

### Layout Patterns
- **Flexbox**: For 1D layouts (rows or columns)
- **CSS Grid**: For 2D layouts (rows AND columns)
- **Container queries**: For component-level responsiveness
- Avoid deep nesting; keep specificity low

### Animations & Transitions
- Use `transform` and `opacity` for performant animations
- Duration: 150ms-300ms for micro-interactions, 300ms-500ms for page transitions
- Easing: `ease-out` for entering, `ease-in` for exiting
- Respect `prefers-reduced-motion` media query

## Accessibility (A11y)
- Semantic HTML: use proper heading levels, landmarks, and ARIA roles
- Keyboard navigation: all interactive elements focusable and operable
- Screen reader support: meaningful alt text, aria-labels
- Color is not the only means of conveying information
- Use `rem` units for scalable text (respect user font size preferences)

## Design System Approach
1. **Tokens**: Define design tokens (colors, spacing, typography) as CSS variables
2. **Atoms**: Build atomic components (Button, Input, Badge)
3. **Molecules**: Compose atoms (SearchBar = Input + Button, FormField = Label + Input + Error)
4. **Organisms**: Compose molecules (Header, Sidebar, DataTable)
5. **Templates**: Page-level layouts combining organisms
6. **Theme**: Support light/dark mode via CSS variable overrides

## Common Pitfalls
- Avoid inline styles; use CSS modules or CSS-in-JS
- Don't use `!important` unless absolutely necessary
- Avoid fixed heights on content containers
- Don't hide focus outlines without providing alternatives
- Beware of z-index wars; use stacking contexts intentionally

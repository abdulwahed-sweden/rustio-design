---
artifact: DESIGN_ARCHITECTURE
layer: what           # WHAT — the structure derived by reasoning from the Brief
status: reserved
updated: ""           # YYYY-MM-DD
---

# Design Architecture — WHAT

> The structural layer between intent and tokens. Reasoned from the Brief; it
> constrains what the token layer must support. Structured enough for Claude
> Design to act on, narrative enough to explain itself. First-class design
> memory — kept in step with the product, not left to drift.

## Information Architecture

_The entities/models, how they group, and their relative priority — the mental
model an operator builds of the system._

- **Domains / groupings:**
- **Primary entities (and why they matter):**
- **Relationships that must be legible in the UI:**
- **Priority order (what leads, what is secondary):**

## Navigation Structure

_How the operator moves: the nav tree, section ordering, and the labels (in the
operator's words, not the schema's)._

```
# example shape — replace with the real tree
Dashboard
Catalogue
  ├─ Products
  └─ Categories
Customers
Orders
  ├─ Orders
  └─ Payments
Settings
```

- **Top-level sections (ordered):**
- **One click away vs deliberately buried:**
- **Terminology / labels:**

## UX Hierarchy

_Per surface: what the eye should hit first, the single primary action, and what
recedes._

- **List pages — emphasis & primary action:**
- **Detail pages — what leads:**
- **Forms — grouping and the one primary action:**
- **Empty / error / loading states — intended tone:**

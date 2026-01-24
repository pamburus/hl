# 🧭 Guidelines for Cross-Feature References in Feature Specifications

## 1. Purpose

Feature specification documents should clearly define **how a single feature works** — its behavior, purpose, inputs, and outputs — without duplicating or redefining logic that belongs to other features.  
Cross-feature references are allowed but must be **intentional, minimal, and clearly scoped**.

---

## 2. General Principles

- **Keep specs cohesive.**  
  Each feature spec should stand alone in describing the feature’s complete behavior.

- **Reference, don’t redefine.**  
  If another feature’s logic is relevant, link to its specification rather than describing it again.

- **Define only necessary integrations.**  
  Include details about another feature **only when your feature depends on it directly** for correct operation.

- **Separate integration concerns.**  
  Broader workflows involving multiple features should be described in a dedicated  
  **“Integration”**, **“Cross-Feature Behavior”**, or **“User Journey”** document.

---

## 3. Decision Framework

Use this table to decide how to handle cross-feature relationships:

| Question | Guidance |
|-----------|-----------|
| **Is the other feature required for this one to function?** | Describe the dependency explicitly and reference the other feature’s spec. |
| **Is the other feature only loosely related or optional?** | Mention it briefly (if at all) and link out. Avoid defining behavior here. |
| **Would understanding this spec require knowing details of the other feature?** | Link to the other spec; do not restate its logic. |
| **Are both features part of one user flow or epic?** | Define the combined behavior in a higher-level integration spec. |

---

## 4. Recommended Structure

When referencing another feature, use a dedicated section near the end of your document:

```markdown
## Interactions with Other Features

- When *Feature A* is enabled, this feature displays an additional tab.  
- For detailed behavior of *Feature A*, see [Feature A Specification](../feature-a/spec.md).  
- No other dependencies or behavioral changes are introduced.
```

This pattern:
- Keeps references discoverable,
- Makes dependencies explicit,
- Prevents duplication or drift between specs.

---

## 5. When Cross-Feature Behavior *Belongs* Here

You may define multi-feature logic inside a single spec **only if**:
- The features are **conceptually inseparable** (e.g., “User Roles” and “Permissions”),
- The spec represents a **shared subsystem** (e.g., unified search across modules),
- The feature’s **primary value emerges from its interaction** with another.

---

## 6. Summary Checklist

✅ Clearly state the feature’s own purpose and scope  
✅ Reference, don’t restate, related features  
✅ Use links for dependencies, not descriptions  
✅ Create or update integration specs for multi-feature workflows  
✅ Keep each feature spec independently understandable

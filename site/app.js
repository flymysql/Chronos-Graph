// Tabbed code blocks
document.querySelectorAll("[data-tabs]").forEach((group) => {
  const buttons = group.querySelectorAll(".tab-btn");
  buttons.forEach((btn) => {
    btn.addEventListener("click", () => {
      const target = btn.getAttribute("data-tab");
      buttons.forEach((b) => b.classList.toggle("active", b === btn));
      group.querySelectorAll(".tab-panel").forEach((panel) => {
        panel.classList.toggle("active", panel.id === target);
      });
    });
  });
});

// Highlight the sidebar link for the section currently in view
const links = Array.from(document.querySelectorAll(".nav-group a[href^='#']"));
const byId = new Map(links.map((a) => [a.getAttribute("href").slice(1), a]));
const sections = links
  .map((a) => document.getElementById(a.getAttribute("href").slice(1)))
  .filter(Boolean);

const observer = new IntersectionObserver(
  (entries) => {
    entries.forEach((entry) => {
      if (entry.isIntersecting) {
        links.forEach((a) => a.classList.remove("active"));
        const active = byId.get(entry.target.id);
        if (active) active.classList.add("active");
      }
    });
  },
  { rootMargin: "-10% 0px -80% 0px", threshold: 0 }
);
sections.forEach((s) => observer.observe(s));

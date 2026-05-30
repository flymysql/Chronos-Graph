// Language toggle (EN / 中文), persisted in localStorage
(function () {
  const KEY = "chronos-lang";
  const toggle = document.getElementById("langToggle");
  if (!toggle) return;
  const buttons = toggle.querySelectorAll("button");

  function apply(lang) {
    document.body.classList.toggle("lang-en", lang === "en");
    document.body.classList.toggle("lang-zh", lang === "zh");
    document.documentElement.lang = lang === "zh" ? "zh-CN" : "en";
    buttons.forEach((b) => b.classList.toggle("active", b.dataset.lang === lang));
  }

  const saved = localStorage.getItem(KEY);
  const prefersZh = (navigator.language || "").toLowerCase().startsWith("zh");
  apply(saved || (prefersZh ? "zh" : "en"));

  buttons.forEach((b) =>
    b.addEventListener("click", () => {
      const lang = b.dataset.lang;
      localStorage.setItem(KEY, lang);
      apply(lang);
    })
  );
})();

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

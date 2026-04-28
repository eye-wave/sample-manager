declare const pagination__: HTMLDivElement;
declare const pagi_template__: HTMLButtonElement;

function createPagination() {
  let current = 1;
  let max = 1;
  const WINDOW = 3;

  const template = pagi_template__.cloneNode(true) as HTMLButtonElement;
  pagi_template__.remove();

  let onClick: null | ((page: number) => void) = null;

  function makeButton(label: string | number, page?: number, cls?: string): HTMLButtonElement {
    const btn = template.cloneNode(true) as HTMLButtonElement;
    btn.textContent = label as unknown as string;
    if (cls) btn.classList.add(cls);
    if (page !== null) {
      btn.id = `page_${page}`;
      if (page !== undefined) {
        btn.onclick = () => {
          current = page;
          onClick?.(page);
          build();
        };
      }
      if (page === current) btn.classList.add("active");
    }
    return btn;
  }

  function makeEllipsis(): HTMLButtonElement {
    const btn = template.cloneNode(true) as HTMLButtonElement;
    btn.textContent = "…";
    btn.classList.add("ellipsis");
    btn.disabled = true;
    return btn;
  }

  function getWindowRange(): { lo: number; hi: number } {
    let lo = current - Math.floor(WINDOW / 2);
    let hi = lo + WINDOW - 1;
    if (lo < 2) {
      lo = 2;
      hi = lo + WINDOW - 1;
    }
    if (hi > max - 1) {
      hi = max - 1;
      lo = hi - WINDOW + 1;
    }
    lo = Math.max(2, lo);
    hi = Math.min(max - 1, hi);
    return { lo, hi };
  }

  function build(): void {
    pagination__.innerHTML = "";

    if (max <= 0) return;

    const prev = makeButton("‹");

    if (current <= 1) {
      prev.disabled = true;
    } else {
      prev.onclick = () => {
        current--;
        onClick?.(current);
        build();
      };
    }
    pagination__.appendChild(prev);

    if (max <= 7) {
      for (let i = 1; i <= max; i++) {
        pagination__.appendChild(makeButton(i, i));
      }
    } else {
      const { lo, hi } = getWindowRange();

      pagination__.appendChild(makeButton(1, 1));
      if (lo > 2) pagination__.appendChild(makeEllipsis());
      for (let i = lo; i <= hi; i++) {
        pagination__.appendChild(makeButton(i, i));
      }
      if (hi < max - 1) pagination__.appendChild(makeEllipsis());
      pagination__.appendChild(makeButton(max, max));
    }

    const next = makeButton("›");

    if (current >= max) {
      next.disabled = true;
    } else {
      next.onclick = () => {
        current++;
        onClick?.(current);
        build();
      };
    }
    pagination__.appendChild(next);
  }

  return {
    set onClick(c: (page: number) => void) {
      onClick = c;
    },

    display(show: boolean) {
      pagination__.style.display = show ? "" : "none";
    },
    setPages(totalPages: number): void {
      max = totalPages;
      current = 1;
      build();
    },

    setPage(p: number): void {
      current = Math.max(1, Math.min(max, p));
      build();
    },

    get page(): number {
      return current;
    },
  };
}

export const PaginationHandler = createPagination();

PaginationHandler.setPages(2);

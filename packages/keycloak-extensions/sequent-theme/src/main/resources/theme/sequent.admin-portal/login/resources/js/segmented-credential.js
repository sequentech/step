// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

const MAX_GROUPS = 8;
const MAX_GROUP_SIZE = 12;
const MAX_TOTAL_SIZE = 64;

const normalizeDigits = (value) => value.replace(/[^0-9]/g, "");

const parseLayout = (value) => {
  const groups = value.split("-");
  if (groups.length < 1 || groups.length > MAX_GROUPS) {
    return null;
  }

  const sizes = [];
  let totalSize = 0;
  for (const group of groups) {
    if (!/^[1-9][0-9]*$/.test(group)) {
      return null;
    }

    const size = Number.parseInt(group, 10);
    if (size > MAX_GROUP_SIZE) {
      return null;
    }

    totalSize += size;
    if (totalSize > MAX_TOTAL_SIZE) {
      return null;
    }
    sizes.push(size);
  }

  return sizes;
};

const formatGroupLabel = (template, groupNumber, groupCount) =>
  template.replaceAll("{0}", String(groupNumber)).replaceAll("{1}", String(groupCount));

const setupVisibilityToggle = (inputs, toggle) => {
  if (!toggle) {
    return;
  }

  const icon = toggle.querySelector("i");
  const labelShow = toggle.dataset.labelShow || "";
  const labelHide = toggle.dataset.labelHide || "";
  const iconShow = toggle.dataset.iconShow || "";
  const iconHide = toggle.dataset.iconHide || "";
  let visible = false;

  toggle.setAttribute("aria-pressed", "false");

  toggle.addEventListener("click", () => {
    visible = !visible;
    for (const input of inputs) {
      input.type = visible ? "text" : "password";
    }
    toggle.setAttribute("aria-label", visible ? labelHide : labelShow);
    toggle.setAttribute("aria-pressed", String(visible));
    if (icon) {
      icon.className = visible ? iconHide : iconShow;
    }
  });
};

const container = document.querySelector("[data-segmented-credential]");

if (container) {
  const realInput = container.querySelector('input[name="password"]');
  const toggle = container.querySelector("[data-segmented-credential-toggle]");
  const layout = parseLayout(container.dataset.segmentLayout || "");

  if (!layout) {
    if (realInput) {
      setupVisibilityToggle([realInput], toggle);
    }
  } else if (realInput && realInput.form) {
    const form = realInput.form;
    const label = document.getElementById(container.dataset.labelId || "");
    const error = document.getElementById(container.dataset.errorId || "");
    const hintId = container.dataset.hintId || "";
    const errorId = container.dataset.errorId || "";
    const groupLabel = container.dataset.groupLabel || "PIN group {0} of {1}";
    const initialValue = normalizeDigits(realInput.value);
    const hadServerError = Boolean(error && !error.hidden);
    const segmentedInput = document.createElement("div");
    const segmentInputs = [];

    segmentedInput.className = "segmented-credential";
    segmentedInput.setAttribute("role", "group");
    if (label) {
      segmentedInput.setAttribute("aria-labelledby", label.id);
    }
    if (hintId || errorId) {
      segmentedInput.setAttribute(
        "aria-describedby",
        [hintId, errorId].filter(Boolean).join(" "),
      );
    }

    const syncPassword = () => {
      realInput.value = segmentInputs.map((input) => input.value).join("");
    };

    const clearError = () => {
      if (error) {
        error.hidden = true;
      }
      segmentedInput.removeAttribute("aria-invalid");
    };

    const showError = () => {
      if (error) {
        error.hidden = false;
      }
      segmentedInput.setAttribute("aria-invalid", "true");
    };

    const focusAfterEntry = (startIndex) => {
      const nextIncompleteIndex = segmentInputs.findIndex(
        (input, index) => index >= startIndex && input.value.length < layout[index],
      );
      const targetIndex = nextIncompleteIndex === -1 ? segmentInputs.length - 1 : nextIncompleteIndex;
      segmentInputs[targetIndex].focus();
      segmentInputs[targetIndex].setSelectionRange(
        segmentInputs[targetIndex].value.length,
        segmentInputs[targetIndex].value.length,
      );
    };

    const distributeDigits = (startIndex, digits, selectionStart) => {
      let remaining = digits;
      const current = segmentInputs[startIndex];
      const prefix = current.value.slice(0, selectionStart);
      const firstCapacity = layout[startIndex] - prefix.length;
      current.value = prefix + remaining.slice(0, firstCapacity);
      remaining = remaining.slice(firstCapacity);

      for (let index = startIndex + 1; index < segmentInputs.length; index += 1) {
        const size = layout[index];
        segmentInputs[index].value = remaining.slice(0, size);
        remaining = remaining.slice(size);
      }

      syncPassword();
      clearError();
      focusAfterEntry(startIndex);
    };

    layout.forEach((size, index) => {
      const segmentInput = document.createElement("input");
      segmentInput.id = `password-segment-${index + 1}`;
      segmentInput.className = `${realInput.className} segmented-credential__segment`;
      segmentInput.type = "password";
      segmentInput.inputMode = "numeric";
      segmentInput.autocomplete = "off";
      segmentInput.autocapitalize = "none";
      segmentInput.spellcheck = false;
      segmentInput.tabIndex = realInput.tabIndex;
      segmentInput.maxLength = size;
      segmentInput.pattern = "[0-9]*";
      segmentInput.setAttribute("aria-required", "true");
      segmentInput.setAttribute(
        "aria-label",
        formatGroupLabel(groupLabel, index + 1, layout.length),
      );
      segmentInput.style.setProperty("--segment-width", `${Math.max(size + 2, 4)}ch`);

      segmentInput.addEventListener("input", () => {
        const digits = normalizeDigits(segmentInput.value).slice(0, size);
        if (segmentInput.value !== digits) {
          segmentInput.value = digits;
        }
        syncPassword();
        clearError();
        if (digits.length === size && index < layout.length - 1) {
          segmentInputs[index + 1].focus();
        }
      });

      segmentInput.addEventListener("paste", (event) => {
        const digits = normalizeDigits(event.clipboardData?.getData("text") || "");
        if (!digits) {
          return;
        }
        event.preventDefault();
        distributeDigits(index, digits, segmentInput.selectionStart || 0);
      });

      segmentInput.addEventListener("keydown", (event) => {
        if (event.key === "Backspace" && segmentInput.value.length === 0 && index > 0) {
          event.preventDefault();
          const previous = segmentInputs[index - 1];
          previous.focus();
          previous.setSelectionRange(previous.value.length, previous.value.length);
        }
      });

      segmentInputs.push(segmentInput);
      segmentedInput.append(segmentInput);
    });

    container.insertBefore(segmentedInput, realInput);
    realInput.type = "hidden";
    realInput.hidden = true;
    realInput.removeAttribute("aria-invalid");
    realInput.removeAttribute("aria-describedby");
    realInput.tabIndex = -1;
    container.dataset.segmentedCredentialEnhanced = "true";

    if (label) {
      label.htmlFor = segmentInputs[0].id;
    }
    if (toggle) {
      toggle.setAttribute("aria-controls", segmentInputs.map((input) => input.id).join(" "));
      setupVisibilityToggle(segmentInputs, toggle);
    }

    if (initialValue) {
      let offset = 0;
      segmentInputs.forEach((input, index) => {
        input.value = initialValue.slice(offset, offset + layout[index]);
        offset += layout[index];
      });
      syncPassword();
    }
    if (hadServerError) {
      showError();
    }

    form.addEventListener(
      "submit",
      (event) => {
        syncPassword();
        const firstIncomplete = segmentInputs.findIndex(
          (input, index) => input.value.length !== layout[index],
        );
        if (firstIncomplete !== -1) {
          event.preventDefault();
          event.stopImmediatePropagation();
          showError();
          segmentInputs[firstIncomplete].focus();
          return;
        }

        const submitter = event.submitter;
        if (submitter) {
          submitter.disabled = true;
        }
      },
      true,
    );

    form.addEventListener("input", (event) => {
      if (!segmentInputs.includes(event.target)) {
        clearError();
      }
    });
  }
}

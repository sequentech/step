// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

const DIGIT_TOKEN = "d";
const MASK_CHARACTER = "*";
const REVEAL_DURATION_MS = 1000;
const MAX_GROUPS = 8;
const MAX_GROUP_SIZE = 12;
const MAX_TOTAL_SIZE = 64;

const onlyAsciiDigits = (value) => (/^[0-9]+$/.test(value) ? value : null);

const pastedDigits = (value) => {
  if (!/^[0-9 \t\r\n\f-]+$/.test(value)) {
    return null;
  }
  const digits = value.replace(/[ \t\r\n\f-]/g, "");
  return digits ? digits : null;
};

const parsePattern = (value) => {
  const tokens = value.split("-");
  if (tokens.length < 1 || tokens.length > MAX_GROUPS) {
    return null;
  }

  const groups = [];
  let digitStart = 0;
  let displayStart = 0;
  for (const token of tokens) {
    if (
      token.length < 1 ||
      token.length > MAX_GROUP_SIZE ||
      !Array.from(token).every((character) => character === DIGIT_TOKEN)
    ) {
      return null;
    }

    if (digitStart + token.length > MAX_TOTAL_SIZE) {
      return null;
    }

    groups.push({
      digitStart,
      displayStart,
      length: token.length,
    });
    digitStart += token.length;
    displayStart += token.length + 1;
  }

  return { groups, source: value, totalSize: digitStart };
};

const parsePlaceholder = (value) => {
  const characters = Array.from(value);
  if (characters.length !== 1) {
    return DIGIT_TOKEN;
  }

  const character = characters[0];
  const codePoint = character.codePointAt(0);
  const isControl =
    codePoint <= 0x1f || (codePoint >= 0x7f && codePoint <= 0x9f);
  return /[0-9\s*-]/u.test(character) || isControl ? DIGIT_TOKEN : character;
};

const formatGroupStatus = (template, groupNumber, groupCount, entered, size) =>
  template
    .split("{0}")
    .join(String(groupNumber))
    .split("{1}")
    .join(String(groupCount))
    .split("{2}")
    .join(String(entered))
    .split("{3}")
    .join(String(size));

const setupVisibilityToggle = (toggle, onChange) => {
  if (!toggle) {
    return;
  }

  const icon = toggle.querySelector("i");
  const labelShow = toggle.dataset.labelShow || "";
  const labelHide = toggle.dataset.labelHide || "";
  const iconShow = toggle.dataset.iconShow || "";
  const iconHide = toggle.dataset.iconHide || "";
  let visible = false;

  const update = () => {
    toggle.setAttribute("aria-label", visible ? labelHide : labelShow);
    toggle.setAttribute("aria-pressed", String(visible));
    if (icon) {
      icon.className = visible ? iconHide : iconShow;
    }
    onChange(visible);
  };

  toggle.addEventListener("click", () => {
    visible = !visible;
    update();
  });
  update();

  return () => {
    visible = false;
    update();
  };
};

const setupNativeVisibilityToggle = (realInput, toggle) => {
  setupVisibilityToggle(toggle, (visible) => {
    realInput.type = visible ? "text" : "password";
  });
};

const container = document.querySelector("[data-structured-credential]");

if (container) {
  const realInput = container.querySelector('input[name="password"]');
  const toggle = container.querySelector("[data-structured-credential-toggle]");
  const pattern = parsePattern(container.dataset.credentialPattern || "");
  const placeholder = parsePlaceholder(
    container.dataset.credentialInputPlaceholder || DIGIT_TOKEN,
  );

  if (!pattern) {
    if (realInput) {
      setupNativeVisibilityToggle(realInput, toggle);
    }
  } else if (realInput && realInput.form) {
    const form = realInput.form;
    const usernameInput = form.querySelector('input[name="username"]');
    const label = document.getElementById(container.dataset.labelId || "");
    const error = document.getElementById(container.dataset.errorId || "");
    const hintId = container.dataset.hintId || "";
    const errorId = container.dataset.errorId || "";
    const groupStatusTemplate =
      container.dataset.groupStatus || "PIN group {0} of {1}, {2} of {3} digits entered";
    const pasteErrorMessage =
      container.dataset.pasteError || "The pasted value does not match the PIN format.";
    const formatErrorMessage =
      container.dataset.formatError || "The entered value does not match the PIN format.";
    const prefilledValue = onlyAsciiDigits(realInput.value);
    const initialValue =
      prefilledValue && prefilledValue.length <= pattern.totalSize ? prefilledValue : "";
    const hadServerError = Boolean(error && !error.hidden);
    const defaultErrorMessage = error?.textContent || "";
    const originalTabIndex = realInput.tabIndex;
    const displayInput = document.createElement("input");
    const status = document.createElement("span");
    const digits = Array(pattern.totalSize).fill(null);
    let activeGroup = 0;
    let replaceGroupOnNextDigit = true;
    let passwordVisible = false;
    let revealedIndex = -1;
    let revealTimer = null;
    let submitting = false;
    let activeSubmitter = null;
    let credentialFormatInvalid = false;
    let hiddenClearPending = false;

    for (let index = 0; index < initialValue.length; index += 1) {
      digits[index] = initialValue[index];
    }

    displayInput.id = "structured-password";
    displayInput.className = `${realInput.className} structured-credential__input`;
    displayInput.type = "text";
    displayInput.inputMode = "numeric";
    displayInput.autocomplete = "current-password";
    displayInput.autocapitalize = "none";
    displayInput.spellcheck = false;
    displayInput.tabIndex = originalTabIndex;
    displayInput.setAttribute("aria-required", "true");
    if (usernameInput) {
      usernameInput.autocomplete = "username";
    }
    if (label) {
      displayInput.setAttribute("aria-labelledby", label.id);
    }

    status.id = "structured-credential-status";
    status.className = "structured-credential__status";
    status.setAttribute("role", "status");
    status.setAttribute("aria-live", "polite");

    const describedBy = [hintId, errorId, status.id].filter(Boolean).join(" ");
    if (describedBy) {
      displayInput.setAttribute("aria-describedby", describedBy);
    }

    const groupEnd = (group) => group.digitStart + group.length;

    const clearReveal = () => {
      if (revealTimer !== null) {
        window.clearTimeout(revealTimer);
        revealTimer = null;
      }
      revealedIndex = -1;
    };

    const displayValue = () => {
      let digitIndex = 0;
      return Array.from(pattern.source)
        .map((token) => {
          if (token !== DIGIT_TOKEN) {
            return token;
          }
          const digit = digits[digitIndex];
          const value =
            digit === null
              ? placeholder
              : passwordVisible || digitIndex === revealedIndex
                ? digit
                : MASK_CHARACTER;
          digitIndex += 1;
          return value;
        })
        .join("");
    };

    const syncPassword = () => {
      realInput.value = digits.map((digit) => digit || "").join("");
    };

    const updateStatus = () => {
      const group = pattern.groups[activeGroup];
      const entered = digits
        .slice(group.digitStart, groupEnd(group))
        .filter((digit) => digit !== null).length;
      status.textContent = formatGroupStatus(
        groupStatusTemplate,
        activeGroup + 1,
        pattern.groups.length,
        entered,
        group.length,
      );
    };

    const applySelection = () => {
      const group = pattern.groups[activeGroup];
      displayInput.setSelectionRange(group.displayStart, group.displayStart + group.length);
    };

    const render = () => {
      displayInput.value = displayValue();
      updateStatus();
      if (document.activeElement === displayInput) {
        applySelection();
      }
    };

    const selectGroup = (index, replaceOnInput = true) => {
      activeGroup = Math.max(0, Math.min(index, pattern.groups.length - 1));
      replaceGroupOnNextDigit = replaceOnInput;
      updateStatus();
      applySelection();
    };

    const firstIncompleteGroup = () => {
      const index = pattern.groups.findIndex((group) =>
        digits.slice(group.digitStart, groupEnd(group)).some((digit) => digit === null),
      );
      return index === -1 ? 0 : index;
    };

    const clearError = () => {
      if (error) {
        error.hidden = true;
        error.textContent = defaultErrorMessage;
      }
      displayInput.removeAttribute("aria-invalid");
      if (usernameInput) {
        usernameInput.removeAttribute("aria-invalid");
      }
      delete container.dataset.structuredCredentialInvalid;
    };

    const showError = (message = defaultErrorMessage) => {
      if (error) {
        error.textContent = message;
        error.hidden = false;
      } else {
        status.textContent = message;
      }
      displayInput.setAttribute("aria-invalid", "true");
      container.dataset.structuredCredentialInvalid = "true";
    };

    const clearCredentialError = () => {
      credentialFormatInvalid = false;
      hiddenClearPending = false;
      clearError();
    };

    const showFormatError = () => {
      credentialFormatInvalid = true;
      showError(formatErrorMessage);
    };

    const revealDigit = (index) => {
      clearReveal();
      if (passwordVisible) {
        return;
      }
      revealedIndex = index;
      revealTimer = window.setTimeout(() => {
        if (revealedIndex === index) {
          revealedIndex = -1;
          revealTimer = null;
          render();
        }
      }, REVEAL_DURATION_MS);
    };

    const clearGroup = (index) => {
      const group = pattern.groups[index];
      digits.fill(null, group.digitStart, groupEnd(group));
    };

    const groupForDigit = (index) =>
      pattern.groups.findIndex(
        (group) => index >= group.digitStart && index < groupEnd(group),
      );

    const enterDigits = (value) => {
      const group = pattern.groups[activeGroup];
      if (replaceGroupOnNextDigit) {
        clearGroup(activeGroup);
        replaceGroupOnNextDigit = false;
      }

      let target = digits.findIndex(
        (digit, index) => index >= group.digitStart && index < groupEnd(group) && digit === null,
      );
      if (target === -1) {
        render();
        return;
      }

      let lastEntered = -1;
      for (const digit of value) {
        if (target >= pattern.totalSize) {
          break;
        }
        digits[target] = digit;
        lastEntered = target;
        target += 1;
      }

      if (lastEntered !== -1) {
        const enteredGroup = groupForDigit(lastEntered);
        const enteredGroupDefinition = pattern.groups[enteredGroup];
        if (
          lastEntered === groupEnd(enteredGroupDefinition) - 1 &&
          enteredGroup < pattern.groups.length - 1
        ) {
          activeGroup = enteredGroup + 1;
          replaceGroupOnNextDigit = true;
        } else {
          activeGroup = enteredGroup;
        }
        revealDigit(lastEntered);
      }

      syncPassword();
      clearCredentialError();
      render();
    };

    const pasteFromActiveGroup = (value) => {
      const start = value.length === pattern.totalSize ? 0 : pattern.groups[activeGroup].digitStart;
      if (value.length > pattern.totalSize - start) {
        return false;
      }

      const lastEntered = start + value.length - 1;
      const enteredGroup = groupForDigit(lastEntered);
      const clearEnd =
        value.length === pattern.totalSize
          ? pattern.totalSize
          : groupEnd(pattern.groups[enteredGroup]);
      digits.fill(null, start, clearEnd);
      for (let index = 0; index < value.length; index += 1) {
        digits[start + index] = value[index];
      }
      activeGroup =
        lastEntered === groupEnd(pattern.groups[enteredGroup]) - 1 &&
        enteredGroup < pattern.groups.length - 1
          ? enteredGroup + 1
          : enteredGroup;
      replaceGroupOnNextDigit = digits
        .slice(
          pattern.groups[activeGroup].digitStart,
          groupEnd(pattern.groups[activeGroup]),
        )
        .every((digit) => digit !== null);
      revealDigit(lastEntered);
      syncPassword();
      clearCredentialError();
      render();
      return true;
    };

    const announcePasteError = () => {
      status.textContent = pasteErrorMessage;
    };

    const deleteBackward = () => {
      const group = pattern.groups[activeGroup];
      const groupIsEmpty = digits
        .slice(group.digitStart, groupEnd(group))
        .every((digit) => digit === null);
      if (groupIsEmpty && activeGroup > 0) {
        clearReveal();
        selectGroup(activeGroup - 1);
        return;
      }

      if (replaceGroupOnNextDigit) {
        clearGroup(activeGroup);
        replaceGroupOnNextDigit = false;
      } else {
        let target = -1;
        for (let index = groupEnd(group) - 1; index >= group.digitStart; index -= 1) {
          if (digits[index] !== null) {
            target = index;
            break;
          }
        }
        if (target === -1 && activeGroup > 0) {
          selectGroup(activeGroup - 1);
          return;
        }
        if (target !== -1) {
          digits[target] = null;
        }
      }
      clearReveal();
      syncPassword();
      clearCredentialError();
      render();
    };

    const deleteGroup = () => {
      clearGroup(activeGroup);
      replaceGroupOnNextDigit = false;
      clearReveal();
      syncPassword();
      clearCredentialError();
      render();
    };

    const groupAtDisplayPosition = (position) => {
      for (let index = 0; index < pattern.groups.length; index += 1) {
        const group = pattern.groups[index];
        if (position <= group.displayStart + group.length) {
          return index;
        }
      }
      return pattern.groups.length - 1;
    };

    displayInput.addEventListener("focus", () => {
      applySelection();
      updateStatus();
    });

    displayInput.addEventListener("click", () => {
      selectGroup(groupAtDisplayPosition(displayInput.selectionStart || 0));
    });

    displayInput.addEventListener("keydown", (event) => {
      const navigation = {
        ArrowLeft: activeGroup - 1,
        ArrowRight: activeGroup + 1,
        Home: 0,
        End: pattern.groups.length - 1,
      };
      if (Object.prototype.hasOwnProperty.call(navigation, event.key)) {
        event.preventDefault();
        selectGroup(navigation[event.key]);
      } else if (event.key === "Backspace") {
        event.preventDefault();
        deleteBackward();
      } else if (event.key === "Delete") {
        event.preventDefault();
        deleteGroup();
      }
    });

    displayInput.addEventListener("beforeinput", (event) => {
      if (event.inputType === "deleteContentBackward") {
        event.preventDefault();
        deleteBackward();
        return;
      }
      if (event.inputType === "deleteContentForward") {
        event.preventDefault();
        deleteGroup();
        return;
      }
      if (event.inputType.startsWith("insert")) {
        // Browser autofill uses replacement input without exposing the value here.
        // Other null-data insertions must not be allowed to overwrite the mask.
        if (event.data == null) {
          if (event.inputType !== "insertReplacementText") {
            event.preventDefault();
          }
          return;
        }
        event.preventDefault();
        const value = onlyAsciiDigits(event.data);
        if (value) {
          enterDigits(value);
        }
        return;
      }
      event.preventDefault();
    });

    displayInput.addEventListener("input", (event) => {
      if (event.data == null) {
        const replacement = pastedDigits(displayInput.value);
        if (
          !replacement ||
          replacement.length !== pattern.totalSize ||
          !pasteFromActiveGroup(replacement)
        ) {
          render();
          showFormatError();
        }
        return;
      }

      const value = onlyAsciiDigits(event.data);
      if (value) {
        enterDigits(value);
      } else {
        render();
      }
    });

    displayInput.addEventListener("paste", (event) => {
      event.preventDefault();
      const value = pastedDigits(event.clipboardData?.getData("text") || "");
      if (!value || !pasteFromActiveGroup(value)) {
        announcePasteError();
      }
    });
    displayInput.addEventListener("copy", (event) => event.preventDefault());
    displayInput.addEventListener("cut", (event) => event.preventDefault());
    displayInput.addEventListener("drop", (event) => event.preventDefault());
    displayInput.addEventListener("blur", () => {
      clearReveal();
      render();
    });

    const importHiddenAutofill = () => {
      const value = onlyAsciiDigits(realInput.value);
      const currentValue = digits.map((digit) => digit || "").join("");
      return Boolean(
        value &&
          value.length === pattern.totalSize &&
          value !== currentValue &&
          pasteFromActiveGroup(value),
      );
    };

    const reconcileHiddenAutofill = (duringSubmit = false) => {
      const suppliedValue = realInput.value;
      const currentValue = digits.map((digit) => digit || "").join("");
      if (suppliedValue === currentValue) {
        if (!duringSubmit && hiddenClearPending && currentValue.length === pattern.totalSize) {
          clearCredentialError();
        }
        return false;
      }
      if (!suppliedValue) {
        if (currentValue) {
          credentialFormatInvalid = true;
          hiddenClearPending = true;
        }
        return duringSubmit;
      }
      if (!importHiddenAutofill()) {
        if (!hiddenClearPending) {
          syncPassword();
        }
        if (!duringSubmit) {
          showFormatError();
        }
        return true;
      }
      return false;
    };

    realInput.addEventListener("input", () => reconcileHiddenAutofill());
    realInput.addEventListener("change", () => reconcileHiddenAutofill());

    activeGroup = firstIncompleteGroup();
    container.insertBefore(displayInput, realInput);
    container.append(status);
    realInput.type = "hidden";
    realInput.hidden = true;
    realInput.removeAttribute("aria-invalid");
    realInput.removeAttribute("aria-describedby");
    realInput.tabIndex = -1;
    container.dataset.structuredCredentialEnhanced = "true";

    if (label) {
      label.htmlFor = displayInput.id;
    }
    let hideCredential = () => {
      passwordVisible = false;
      clearReveal();
      render();
    };
    if (toggle) {
      toggle.setAttribute("aria-controls", displayInput.id);
      toggle.tabIndex = originalTabIndex;
      toggle.addEventListener("pointerdown", (event) => event.preventDefault());
      hideCredential =
        setupVisibilityToggle(toggle, (visible) => {
          passwordVisible = visible;
          clearReveal();
          render();
        }) || hideCredential;
    }

    document.addEventListener("visibilitychange", () => {
      if (document.hidden) {
        hideCredential();
      }
    });
    window.addEventListener("pagehide", hideCredential);

    const resetSubmission = () => {
      submitting = false;
      if (activeSubmitter) {
        activeSubmitter.disabled = false;
        activeSubmitter = null;
      }
    };
    window.addEventListener("pageshow", resetSubmission);

    syncPassword();
    render();
    if (hadServerError) {
      showError();
    }

    // The template puts autofocus on the real input, which this widget hides - a hidden field
    // cannot hold focus, so the page would open with nothing focused. Move the intent onto the
    // visible group input.
    if (realInput.hasAttribute("autofocus")) {
      realInput.removeAttribute("autofocus");
      displayInput.focus();
      selectGroup(activeGroup);
    }

    form.addEventListener(
      "submit",
      (event) => {
        const visibleReplacement = displayInput.value !== displayValue();
        const visibleAutofill = visibleReplacement ? pastedDigits(displayInput.value) : null;
        let invalidReplacement = false;
        if (
          visibleReplacement &&
          visibleAutofill &&
          visibleAutofill.length === pattern.totalSize
        ) {
          pasteFromActiveGroup(visibleAutofill);
        } else if (visibleReplacement) {
          render();
          invalidReplacement = true;
        } else {
          invalidReplacement = reconcileHiddenAutofill(true);
        }
        const incomplete = pattern.groups.findIndex((group) =>
          digits.slice(group.digitStart, groupEnd(group)).some((digit) => digit === null),
        );
        if (incomplete !== -1 || invalidReplacement || credentialFormatInvalid) {
          event.preventDefault();
          event.stopImmediatePropagation();
          displayInput.focus();
          selectGroup(incomplete !== -1 ? incomplete : activeGroup);
          if (invalidReplacement || credentialFormatInvalid) {
            showFormatError();
          } else {
            showError();
          }
          return;
        }

        syncPassword();

        if (submitting) {
          event.preventDefault();
          event.stopImmediatePropagation();
          return;
        }
        submitting = true;
        activeSubmitter =
          event.submitter instanceof HTMLButtonElement ||
          event.submitter instanceof HTMLInputElement
            ? event.submitter
            : null;
        window.setTimeout(() => {
          if (event.defaultPrevented) {
            resetSubmission();
          } else if (submitting && activeSubmitter) {
            activeSubmitter.disabled = true;
          }
        }, 0);
      },
      true,
    );

    form.addEventListener("input", (event) => {
      if (event.target === usernameInput) {
        usernameInput.removeAttribute("aria-invalid");
        if (!credentialFormatInvalid) {
          clearError();
        }
      }
    });
  }
}

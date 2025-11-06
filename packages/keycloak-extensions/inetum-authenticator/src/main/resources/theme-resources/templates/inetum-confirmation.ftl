<#--
 SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
    SPDX-License-Identifier: AGPL-3.0-only
    -->
    <#import "template.ftl" as layout>
        <@layout.registrationLayout displayMessage=false displayCard=false; section>
            <#if section="head">
                <link href="${url.resourcesPath}/inetum-sdk-${sdk_version}/assets/css/dob-styles.css" rel="stylesheet" />
                <link rel="preconnect" href="https://fonts.googleapis.com" />
                <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
                <link
                    href="https://fonts.googleapis.com/css2?family=Poppins:ital,wght@0,100;0,200;0,300;0,400;0,500;0,600;0,700;0,800;0,900;1,100;1,200;1,300;1,400;1,500;1,600;1,700;1,800;1,900&display=swap"
                    rel="stylesheet" />
                <style>
                .size-6 {
                    width: 1.5rem;
                    /* 24px */
                    height: 1.5rem;
                    /* 24px */
                }
                </style>
                <#--  Disable submit button after submit  -->
                <script>
                let submittingText = "${msg('ButtonSubmitting')}";

                document.addEventListener("DOMContentLoaded", () => {
                    const form = document.querySelector("form");
                    const submitButton = form.querySelector("button[type='submit']");

                    form.addEventListener("submit", (event) => {
                        // Disable the button to prevent multiple submissions
                        if (submitButton.disabled) {
                            event.preventDefault(); // Prevent form submission if already clicked
                        } else {
                            submitButton.disabled = true;
                            submitButton.textContent = submittingText; // Optional: Change the button text
                        }
                    });
                });
                </script>
                <#elseif section="body">
                    <div class="dob-sdk-container dob-sdk-root-container" style="height: auto; background-color: white;">
                            <main style="margin: 32px">
                                <h1 style="font-family: Dob-Font-Bold; color: #00aa9b">
                                    ${msg("ConfirmationTitle")}
                                </h1>
                                <p style="font-family: Dob-Font-Bold; color: #242b4; font-size: 24px">
                                    ${msg("ConfirmationDescription")}
                                </p>
                                <div
                                    style="
                                        display: flex;
                                        flex-direction: column;
                                        justify-content: space-between;
                                        margin: 4rem 0;
                                        ">
                                    <#list storedAttributes as attribute>
                                        <div style="margin: 8px; 0; display: flex; flex-direction: column">
                                            <label style="font-family: Dob-Font-Bold; font-size: 16px">
                                                ${msg(attribute["key"])}
                                            </label>
                                            <input tabindex="1" id='${attribute["key"]}' class="${properties.kcInputClass!}" name='${attribute["key"]}' type='${attribute["type"]}' autofocus autocomplete="off"
                                                value='${attribute["value"]!"-"}' disabled style="padding: 8px 16px; flex: 1" />
                                        </div>
                                    </#list>
                                </div>
                                <form action="${actionUrl}" method="post">
                                <input type="hidden" name="action" value="confirm" />
                                <div class="confirmation-buttons-wrapper">
                                    <div class="confirmation-buttons-container">
                                        <button type="submit" class="confirmation-button" id="confirmation-button">
                                            <span>${msg("ButtonContinue")}</span>
                                            <svg
                                                xmlns="http://www.w3.org/2000/svg"
                                                fill="none"
                                                viewBox="0 0 24 24"
                                                stroke-width="1.5"
                                                stroke="white"
                                                class="size-6">
                                                <path
                                                    stroke-linecap="round"
                                                    stroke-linejoin="round"
                                                    d="m4.5 12.75 6 6 9-13.5" />
                                            </svg>
                                        </button>
                                    </div>
                                    <div
                                        class="repeat-button-container">
                                        <button onclick="location.reload(); return false;" 
                                            class="repeat-button">
                                            <span>${msg("ButtonRepeat")}</span>
                                            <svg
                                                xmlns="http://www.w3.org/2000/svg"
                                                fill="none"
                                                viewBox="0 0 24 24"
                                                stroke-width="1.5"
                                                stroke="white"
                                                class="size-6">
                                                <path
                                                    stroke-linecap="round"
                                                    stroke-linejoin="round"
                                                    d="M6 18 18 6M6 6l12 12" />
                                            </svg>
                                        </button>
                                    </div>
                                </div>
                                </form>
                            </main>
                    </div>
            </#if>
        </@layout.registrationLayout>
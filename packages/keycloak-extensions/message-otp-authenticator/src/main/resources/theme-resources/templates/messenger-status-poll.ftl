<#--
 SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->
<#--
 Asks every few seconds whether the voter opened the Messenger chat, so the page moves on once the
 code was sent. It stops when the code was sent, when the code's lifetime is over, while the tab is
 hidden and while the voter is typing a code.
-->
<form id="kc-messenger-poll-form" action="${url.loginAction}" method="post" hidden>
    <input type="hidden" name="messengerStatus" value="true" />
</form>
<script>
    (function () {
        var state = "${(messengerState!'')?js_string}";
        var link = "${(messengerLink!'')?js_string}";
        var lifetimeMs = ${((ttl!'300')?number)?c} * 1000;
        <#noparse>
        var intervalMs = 5000;
        if (state !== "PENDING" || !link) {
            return;
        }
        var key = "messengerPollStart:" + link;
        var started = Number(sessionStorage.getItem(key));
        if (!started) {
            started = Date.now();
            sessionStorage.setItem(key, String(started));
        }
        var loaded = Date.now();
        var timer = setInterval(check, intervalMs);
        function check() {
            if (Date.now() - started > lifetimeMs) {
                clearInterval(timer);
                return;
            }
            var typing = Array.prototype.some.call(
                document.querySelectorAll(".otp-input"),
                function (input) { return input.value !== ""; }
            );
            if (document.hidden || typing || Date.now() - loaded < intervalMs) {
                return;
            }
            clearInterval(timer);
            document.getElementById("kc-messenger-poll-form").submit();
        }
        document.addEventListener("visibilitychange", check);
        </#noparse>
    })();
</script>

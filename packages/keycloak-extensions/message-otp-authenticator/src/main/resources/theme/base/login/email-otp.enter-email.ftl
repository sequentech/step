<#-- email-otp.enter-email.ftl -->
<!DOCTYPE html>
<html>
<head>
    <title>Email Verification</title>
</head>
<body>
    <h2>${msg("emailOtp.auth.enterEmailTitle")}</h2>
    <#if error??>
        <div class="error">${error}</div>
    </#if>
    <form method="post">
        <label for="email">${msg("emailOtp.auth.enterEmailLabel")}</label>
        <input type="email" id="email" name="email" required autofocus />
        <button type="submit">${msg("emailOtp.auth.sendCodeButton")}</button>
    </form>
</body>
</html>

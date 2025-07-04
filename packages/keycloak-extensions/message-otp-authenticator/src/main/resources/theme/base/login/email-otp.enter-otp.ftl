<#-- email-otp.enter-otp.ftl -->
<!DOCTYPE html>
<html>
<head>
    <title>Email OTP Verification</title>
</head>
<body>
    <h2>${msg("emailOtp.auth.enterOtpTitle")}</h2>
    <#if error??>
        <div class="error">${error}</div>
    </#if>
    <form method="post">
        <div>${msg("emailOtp.auth.otpSentTo")}: <b>${email}</b></div>
        <label for="otp">${msg("emailOtp.auth.enterOtpLabel")}</label>
        <input type="text" id="otp" name="otp" required autofocus maxlength="6" pattern="\\d{6}" />
        <button type="submit">${msg("emailOtp.auth.verifyButton")}</button>
    </form>
</body>
</html>

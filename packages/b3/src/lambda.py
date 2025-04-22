import boto3
import os
from datetime import datetime, timedelta, timezone
from botocore.exceptions import ClientError

# --- Configuration (Set as Lambda Environment Variables) ---
# The ARN of the SNS topic to send the alarm notification to
SNS_ALARM_TOPIC_ARN = os.environ.get('SNS_ALARM_TOPIC_ARN')
# The percentage threshold for *both* alarms (SMS spending and Email quota)
THRESHOLD_PERCENTAGE = float(os.environ.get('THRESHOLD_PERCENTAGE', "95")) # Default to 95%
# The currency unit expected (usually USD) - primarily for SMS spending check
CURRENCY_UNIT = os.environ.get('CURRENCY_UNIT', 'USD')

# NEW: The name of the CloudWatch Log Group where your SNS SMS delivery statuses are logged.
# This MUST be set in your Lambda environment variables.
SMS_DELIVERY_LOG_GROUP_NAME = os.environ.get('SMS_DELIVERY_LOG_GROUP_NAME')

# --- AWS Clients ---
sns_client = boto3.client('sns')
ce_client = boto3.client('ce')
ses_client = boto3.client('ses') 
cloudwatch_client = boto3.client('cloudwatch')
logs_client = boto3.client('logs')

# --- Existing Function: Get SNS SMS Spend Limit ---
def get_sns_sms_monthly_spend_limit():
    """Reads the configured monthly spending limit for SNS SMS."""
    try:
        # Note: This limit is account-specific and managed in SNS settings
        response = sns_client.get_sms_attributes(
            attributes=['MonthlySpendLimit']
        )
        limit_str = response.get('attributes', {}).get('MonthlySpendLimit')

        if limit_str is None:
             print("Warning: MonthlySpendLimit not found in SNS SMS attributes. SMS limit check skipped.")
             return None # Indicate limit not found

        limit_amount = float(limit_str)

        print(f"Found SNS SMS monthly spend limit: {limit_amount} {CURRENCY_UNIT}")
        return limit_amount

    except ClientError as e:
        print(f"Error reading SNS SMS attributes: {e}")
        # Do not re-raise, allow other checks to potentially run
        return None


# --- Existing Function: Get Month-to-Date SNS SMS Spending ---
def get_month_to_date_sns_sms_spending():
    """Gets the current month-to-date SMS spending from CloudWatch 'SMSMonthToDateSpentUSD' metric using GetMetricData."""
    try:
        # CloudWatch metrics are typically in UTC
        end_time = datetime.now(timezone.utc)
        # Query a short time window to get the latest datapoint
        # The metric is cumulative for the month, so the latest value is what we need.
        start_time = end_time - timedelta(minutes=10) # Query last 10 minutes

        response = cloudwatch_client.get_metric_data(
            MetricDataQueries=[
                {
                    'Id': 'smsmtdspent', # Unique ID for this query
                    'MetricStat': {
                        'Metric': {
                            'Namespace': 'AWS/SNS',
                            'MetricName': 'SMSMonthToDateSpentUSD',
                            # No Dimensions needed for this specific metric
                        },
                        'Period': 60, # Aggregate into 1-minute periods (should align with metric emission)
                        'Stat': 'Maximum', # Get the maximum reported value in each period
                    },
                    'ReturnData': True, # We want the data back
                },
            ],
            StartTime=start_time,
            EndTime=end_time,
            # Optional: ScanBy='TimestampDescending' might be useful if processing many results,
            # but with a short time window, the latest is usually the last one.
        )

        spending = 0.0
        # Find the result for our query ID
        metric_data_results = response.get('MetricDataResults', [])
        for result in metric_data_results:
            if result.get('Id') == 'smsmtdspent':
                # Get the latest value from the datapoints returned
                # GetMetricData returns data points in ascending order by timestamp by default
                if result.get('Values'):
                    spending = float(result['Values'][-1]) # The last value is the latest one
                # If no datapoints, spending remains 0.0

        print(f"CloudWatch SMS Month-to-Date Spending: {spending} USD")
        return spending

    except ClientError as e:
        print(f"Error getting SNS SMS spending from CloudWatch: {e}")
        return 0.0

# --- NEW Function: Check SES Daily Email Quota ---
def check_ses_email_quota(threshold_percentage, sns_topic_arn):
    """Checks the SES daily sending quota and usage and sends an alarm if over threshold."""
    try:
        # Get SES sending limits and usage
        response = ses_client.get_send_quota()

        daily_quota = response.get('Max24HourSend')
        sent_last_24_hours = response.get('SentLast24Hours')

        if daily_quota is None or sent_last_24_hours is None:
             print("Warning: Could not retrieve SES daily quota or sent count. SES email check skipped.")
             return # Cannot perform check

        # Convert to float just in case, though documentation suggests they are already
        daily_quota = float(daily_quota)
        sent_last_24_hours = float(sent_last_24_hours)

        if daily_quota <= 0:
             print("Warning: SES daily quota is 0 or less. Cannot perform percentage comparison.")
             # Decide how to handle this case - maybe always alert if usage > 0?
             # For now, just print warning and skip percentage comparison.
             return

        # Calculate threshold
        alarm_threshold = daily_quota * (threshold_percentage / 100.0)

        print(f"SES Daily Email Quota: {daily_quota}")
        print(f"SES Sent Last 24 Hours: {sent_last_24_hours}")
        print(f"SES Alarm Threshold ({threshold_percentage}%): {alarm_threshold}")


        # Compare usage to threshold
        if sent_last_24_hours >= alarm_threshold:
            print(f"ALERT: SES sent count ({sent_last_24_hours}) is at or above the {threshold_percentage}% threshold ({alarm_threshold}) of the daily quota ({daily_quota}).")

            subject = f"AWS SES Email Quota Alert - {threshold_percentage}% Threshold Reached"
            # Prevent division by zero if sent_last_24_hours is 0 but threshold > 0
            percentage_used = (sent_last_24_hours / daily_quota) * 100 if daily_quota > 0 else float('inf')

            message = (
                f"Your AWS SES account has sent {sent_last_24_hours} emails in the last 24 hours.\n"
                f"This is approximately {percentage_used:.2f}% of your daily sending quota of {daily_quota}.\n"
                f"The alarm threshold was set at {threshold_percentage}% ({alarm_threshold})."
            )
            send_alarm_sns(sns_topic_arn, subject, message)
        else:
            print(f"SES sent count ({sent_last_24_hours}) is below the {threshold_percentage}% threshold ({alarm_threshold}). No alarm sent.")

    except ClientError as e:
        print(f"Error checking SES email quota: {e}")
        # Do not re-raise, allow other checks to proceed
        return

# --- NEW Function: Check CloudWatch Logs for Quota Errors ---
def check_sms_quota_logs(log_group_name, sns_topic_arn):
    """Checks CloudWatch Logs for messages indicating SMS quota has been hit."""
    if not log_group_name:
        print("Warning: SMS_DELIVERY_LOG_GROUP_NAME environment variable not set. SMS quota log check skipped.")
        return

    try:
        # Query logs for the last, say, 10 minutes (same as CW metric window)
        # The start and end times for filter_log_events are in milliseconds
        end_time_ms = int(datetime.now(timezone.utc).timestamp() * 1000)
        start_time_ms = int((datetime.now(timezone.utc) - timedelta(minutes=10)).timestamp() * 1000)

        # Pattern to search for. CloudWatch Logs patterns are powerful, but a simple string match is enough here.
        # We escape potential special characters in the literal string.
        filter_pattern = '"{No quota left for account}"' # Match the exact phrase

        print(f"Checking log group '{log_group_name}' for pattern '{filter_pattern}' in last 10 minutes.")

        response = logs_client.filter_log_events(
            logGroupName=log_group_name,
            startTime=start_time_ms,
            endTime=end_time_ms,
            filterPattern=filter_pattern,
            limit=5 # Limit the number of events returned, we just need to know if *any* exist
        )

        # If any events are found, it means the message appeared in the logs
        if response.get('events'):
            print(f"ALERT: Found {len(response['events'])} log events matching '{filter_pattern}' in '{log_group_name}'.")
            # We found the log message indicating quota was hit. Send an alarm.

            subject = "AWS SNS SMS Hard Quota Hit - Log Alert"
            message = (
                f"The specific log message '{filter_pattern}' was found in CloudWatch Log Group '{log_group_name}' "
                f"within the last 10 minutes. This indicates that SMS sending attempts are failing due to the account's "
                f"hard monthly spending limit being reached.\n\n"
                f"Check your SNS Text Messaging preferences and CloudWatch Logs."
            )
            send_alarm_sns(sns_topic_arn, subject, message)
        else:
            print(f"No log events matching '{filter_pattern}' found in '{log_group_name}' in the last 10 minutes.")


    except ClientError as e:
        print(f"Error filtering CloudWatch Logs for quota errors: {e}")
        return
    except Exception as e:
        print(f"An unexpected error occurred during the SMS quota log check: {type(e).__name__}: {e}")
        return

# --- Existing Function: Send Alarm SNS ---
def send_alarm_sns(topic_arn, subject, message):
    """Publishes a message to the specified SNS topic."""
    try:
        sns_client.publish(
            TopicArn=topic_arn,
            Subject=subject,
            Message=message
        )
        print(f"Successfully sent alarm to SNS topic: {topic_arn}")
    except ClientError as e:
        print(f"Error sending SNS alarm: {e}")
        # We don't re-raise here as the primary task (checking) completed


# --- Modified Lambda Handler ---
def lambda_handler(event, context):
    """Main Lambda function handler."""
    if not SNS_ALARM_TOPIC_ARN:
        print("Configuration error: SNS_ALARM_TOPIC_ARN environment variable is not set.")
        return {
            'statusCode': 500,
            'body': 'Configuration error: Missing SNS_ALARM_TOPIC_ARN environment variable.'
        }

    print("Starting spending and quota checks...")

    # --- Perform SNS SMS Spending Check ---
    print("\n--- Checking SNS SMS Spending ---")
    try:
        sms_spend_limit = get_sns_sms_monthly_spend_limit()

        # Only proceed with comparison if SMS limit was successfully retrieved
        if sms_spend_limit is not None:
            current_sms_spending = get_month_to_date_sns_sms_spending()
            limit_100k = 100e3
            if current_sms_spending > limit_100k:
                alarm_threshold_sms = limit_100k + (sms_spend_limit - limit_100k) * (THRESHOLD_PERCENTAGE / 100.0)
            else:
                alarm_threshold_sms = sms_spend_limit * (THRESHOLD_PERCENTAGE / 100.0)

            print(f"Comparing SNS SMS Spending ({current_sms_spending}) to Threshold ({alarm_threshold_sms})")
            if current_sms_spending >= alarm_threshold_sms:
                print(f"ALERT Condition Met for SNS SMS Spending.")
                subject_sms = f"AWS SNS SMS Spending Alert - {THRESHOLD_PERCENTAGE}% Threshold Reached"
                # Prevent division by zero if limit is 0 but spending > 0
                percentage_used_sms = (current_sms_spending / sms_spend_limit) * 100 if sms_spend_limit > 0 else float('inf')

                message_sms = (
                    f"Your AWS SNS SMS spending for the current month has reached {current_sms_spending} {CURRENCY_UNIT}.\n"
                    f"This is approximately {percentage_used_sms:.2f}% of your monthly SMS spending limit of {sms_spend_limit} {CURRENCY_UNIT}.\n"
                    f"The alarm threshold was set at {THRESHOLD_PERCENTAGE}% ({alarm_threshold_sms} {CURRENCY_UNIT}).\n\n"
                    f"Check your SNS SMS spending in the AWS Cost Explorer."
                )
                send_alarm_sns(SNS_ALARM_TOPIC_ARN, subject_sms, message_sms)
            else:
                print("SNS SMS Spending is below threshold.")
        else:
             print("SNS SMS Limit not retrieved, skipping spending comparison.")

    except Exception as e:
        print(f"An error occurred during the SNS SMS check: {e}")
        # Allow the SES check to potentially run even if SMS check fails

    # --- Perform SES Email Quota Check ---
    print("\n--- Checking SES Email Quota ---")
    try:
        check_ses_email_quota(THRESHOLD_PERCENTAGE, SNS_ALARM_TOPIC_ARN)
    except Exception as e:
        print(f"An error occurred during the SES Email check: {e}")
        # Allow Lambda to finish even if SES check fails

    # --- Perform NEW CloudWatch Log Check for SMS Quota Errors ---
    print("\n--- Checking CloudWatch Logs for SMS Quota Errors ---")
    try:
        check_sms_quota_logs(SMS_DELIVERY_LOG_GROUP_NAME, SNS_ALARM_TOPIC_ARN)
    except Exception as e:
         print(f"An error occurred during the SMS quota log check: {type(e).__name__}: {e}")

    print("\nAll checks completed.")

    return {
        'statusCode': 200,
        'body': 'Spending and quota checks completed.'
    }
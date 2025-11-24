#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
# 
# SPDX-License-Identifier: AGPL-3.0-only


import json
import argparse

def update_template(template_path, config_path):
    # Load the contents of the template.hbs file
    with open(template_path, 'r') as template_file:
        template_content = template_file.read()

    # Load the velvet-config.json file
    with open(config_path, 'r') as config_file:
        config_data = json.load(config_file)

    # Update the template content in the vote-receipts pipe config
    for stage in config_data['stages']['main']['pipeline']:
        if stage['id'] == 'vote-receipts':
            stage['config']['template'] = template_content
            break

    # Save the updated config back to velvet-config.json
    with open(config_path, 'w') as config_file:
        json.dump(config_data, config_file, indent=4)

    print("Template has been updated in velvet-config.json")

def main():
    parser = argparse.ArgumentParser(description='Update the template in velvet-config.json')
    parser.add_argument('--template-path', type=str, help='Path to the template.hbs file')
    parser.add_argument('--config-path', type=str, help='Path to the velvet-config.json file')
    args = parser.parse_args()

    update_template(args.template_path, args.config_path)

if __name__ == '__main__':
    main()

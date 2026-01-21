// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

$(document).ready(function(){
    $('#password').on("keyup", function(){
        let result = zxcvbn(this.value, user_inputs=[]);
        let classes = ['pf-m-danger', 'pf-m-warning', 'pf-m-warning', 'pf-m-success', 'pf-m-success']

        for (const element of classes) {
            $('#password-progress').removeClass(element);
        }

        let score = (result.score + 1) * 20;
        
        if(this.value == null | this.value == "") {
            $('#password-progress-indicator').attr('aria-valuenow', 0).css('width', '0%');
        } else {
            $('#password-progress').addClass(classes[result.score]);
            $('#password-progress-indicator').attr('aria-valuenow', score).css('width', score + '%');
        }
    });
});
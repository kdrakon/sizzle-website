$(document).ready(function() {

  $('.js-form-container input').on('focus', function(e) {
    $('.note-block').hide()
  });

  $('.js-form-container form').on('submit', function() {
    var form = $(this);
    var container = form.parent();
    var email = form.find('.js-email-input').eq(0);
    var regex = /^([a-zA-Z0-9_.+-])+\@(([a-zA-Z0-9-])+\.)+([a-zA-Z0-9]{2,6})+$/;
    if ((regex.test(email.val()) == false) || (email.val().length == 0)) {
      container.find('.note-block.w-form-fail').show();
      return false;
    }
  })
});
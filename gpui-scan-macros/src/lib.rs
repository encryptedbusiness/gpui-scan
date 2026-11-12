//! Attribute macros for `gpui-scan`.

use proc_macro::TokenStream;
use quote::quote;
use syn::{ImplItem, ImplItemFn, ItemImpl, parse_macro_input};

/// Tracks every render of the annotated `Render` implementation.
///
/// The attribute wraps the method body, so views are reported by type name without a call-site
/// wrapper. Idle renders only pay one signal lookup; the tracking element is built only on the
/// frame the view was notified.
///
/// ```ignore
/// #[gpui_scan::scan]
/// impl Render for Counter {
///   fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
///     div().child(self.count.to_string())
///   }
/// }
/// ```
#[proc_macro_attribute]
pub fn scan(_attr: TokenStream, item: TokenStream) -> TokenStream {
  let mut item_impl = parse_macro_input!(item as ItemImpl);

  for item in &mut item_impl.items {
    let ImplItem::Fn(method) = item else {
      continue;
    };
    if method.sig.ident != "render" || !returns_impl_trait(method) {
      continue;
    }
    let Some(label) = type_label(&item_impl.self_ty) else {
      continue;
    };
    let Some(context) = context_ident(method) else {
      continue;
    };

    let body = method.block.clone();
    let body = quote! {{
      let __gpui_scan_signal = ::gpui_scan::tracking::begin(#context);
      let __gpui_scan_element = #body;
      ::gpui_scan::tracking::finish(
        __gpui_scan_signal,
        __gpui_scan_element,
        #label,
        file!(),
        line!(),
        column!(),
      )
    }};
    method.block = syn::parse2(body).expect("the generated scan block parses");
  }

  quote!(#item_impl).into()
}

fn returns_impl_trait(method: &ImplItemFn) -> bool {
  match &method.sig.output {
    syn::ReturnType::Default => true,
    syn::ReturnType::Type(_, ty) => matches!(&**ty, syn::Type::ImplTrait(_)),
  }
}

fn context_ident(method: &ImplItemFn) -> Option<syn::Ident> {
  let mut typed = method.sig.inputs.iter().filter_map(|arg| match arg {
    syn::FnArg::Typed(pat_type) => Some(pat_type),
    syn::FnArg::Receiver(_) => None,
  });
  typed.next()?;
  let context = typed.next()?;
  match &*context.pat {
    syn::Pat::Ident(pattern) => Some(pattern.ident.clone()),
    _ => None,
  }
}

fn type_label(ty: &syn::Type) -> Option<String> {
  let syn::Type::Path(path) = ty else {
    return None;
  };
  path
    .path
    .segments
    .last()
    .map(|segment| segment.ident.to_string())
}

use crate::{database, svg};
use maud::{html, Markup, DOCTYPE};
use std::{collections::HashMap, ops::Range};

fn get_pagination(
    number_of_pages: usize,
    current_page: usize,
    displayed_pages: usize,
) -> Range<usize> {
    if number_of_pages < displayed_pages {
        0..number_of_pages
    } else {
        if current_page < number_of_pages - displayed_pages / 2 {
            let low = current_page
                .checked_sub(displayed_pages / 2)
                .unwrap_or_default();
            low..low + displayed_pages
        } else {
            number_of_pages - displayed_pages..number_of_pages
        }
    }
}

fn get_query(params: &HashMap<&str, String>) -> Option<String> {
    params
        .into_iter()
        .filter(|(_, v)| !v.is_empty() && *v != "0")
        .map(|(k, v)| format!("{}={}", k, v))
        .reduce(|acc, s| format!("{}&{}", acc, s))
        .map(|s| format!("?{}", s))
}

fn pagination<T>(page: database::Page<T>) -> Markup {
    let mut params = HashMap::new();
    params.insert("search", page.query.unwrap_or_default());
    html! {
        @if page.number_of_pages>1
        {
            div class="flex flex-row gap-4 justify-center mt-4 text-black" {
                @let button_style = " grid justify-center content-center size-8 rounded-full";
                @if page.current_page==0 {
                    div class={"bg-zinc-700" (button_style)} {
                        div class="size-6"{
                            (svg::left_arrow())
                        }
                    }
                }
                @else {
                    a hx-target="#content" hx-boost="true" href={(page.target) ({params.insert("page",(page.current_page-1).to_string());get_query(&params).unwrap_or_default()})} class={"bg-violet-400 hover:bg-black hover:text-white" (button_style)} {
                        div class="size-6"{
                            (svg::left_arrow())
                        }
                    }
                }
                @for p in get_pagination(page.number_of_pages as usize,page.current_page as usize,5) {
                    a hx-target="#content" hx-boost="true" href={(page.target) ({params.insert("page",p.to_string());get_query(&params).unwrap_or_default()})} hx-push-url="true" class={"hover:bg-black hover:text-white " @if p==page.current_page as usize {"bg-violet-400"} @else {"bg-white"} (button_style)} {
                        (p+1)
                    }
                }
                @for _ in 0..5usize.checked_sub(page.number_of_pages as usize).unwrap_or_default() {
                    div class={"bg-zinc-700" (button_style)} {}
                }
                @if page.current_page==page.number_of_pages-1 {
                    div class={"bg-zinc-700" (button_style)} {
                        div class="size-6"{
                            (svg::right_arrow())
                        }
                    }
                }
                @else {
                    a hx-target="#content" hx-boost="true" href={(page.target) ({params.insert("page",(page.current_page+1).to_string());get_query(&params).unwrap_or_default()})}  class={"bg-violet-400 hover:bg-black hover:text-white" (button_style)} {
                        div class="size-6"{
                            (svg::right_arrow())
                        }
                    }
                }
            }
        }
    }
}

pub fn item_page(
    item: &database::Item,
    page: Option<database::Page<database::RatingItem>>,
    user: Option<&database::User>,
    rating: Option<i16>,
) -> Markup {
    let rating = rating.unwrap_or_default();
    html! {
        @if let Some(user) = user {
            @if user.is_admin {
                div class="mb-4 flex flex-row gap-x-4" {
                    button hx-get={"/items/" (item.locator) "/edit"} hx-swap="afterend" class="rounded-full p-2 bg-violet-400 hover:bg-black hover:text-white" {
                        "Edit item"
                    }
                    button hx-get={"/items/" (item.locator) "/remove"} hx-swap="afterend"  class="rounded-full p-2 bg-violet-400 hover:bg-black hover:text-white" {
                        "Remove item"
                    }
                }
            }
        }
        div class="flex flex-row [@media(max-width:39rem)]:flex-col gap-4" {
            div {
                div style={"background-image: url('/static/images/items/" (item.locator) "')"} class="flex-none w-64 aspect-[3/4] rounded-md bg-cover bg-center" {}
            }
            div class="text-white" {
                b class="text-2xl" {
                    (item.title)
                }
                br;
                "Score: " b class="text-violet-400" {(format!("{:.2}",item.score)) "/10.00 (#" (item.rank) ")"}
                " Reviews: " b class="text-violet-400" {(item.review_count) " (#" (item.popularity) ")"}
                br;
                br;
                b {
                    "Your rating"
                    @if user.is_some() && rating!=0 {
                        " "
                        button hx-delete={"/items/" (item.locator) "/rate"} {
                            span class="px-2 text-xs bg-zinc-700" {
                                "Remove review"
                            }
                        }
                    }
                }
                @if user.is_some() {
                    div class="relative z-0 flex flex-row size-fit group" {
                        @if rating==0 {
                            div class="absolute left-1/2 top-1/2 translate-x-[-50%] translate-y-[-50%] text-white select-none group-hover:hidden" {
                                "Item not rated yet"
                            }
                        }
                        @for s in 0..5 {
                            button hx-post={"/items/" (item.locator) "/rate"} hx-target="#content" name="score" value={(2*s+1)} class={"peer peer-hover:text-zinc-700 w-8" @if (2*s+1)<=rating {" text-yellow-400"} @else {" text-zinc-700 group-hover:text-yellow-400"}} {
                                (svg::star_left())
                            }
                            button hx-post={"/items/" (item.locator) "/rate"} hx-target="#content" name="score" value={(2*s+2)} class={"peer peer-hover:text-zinc-700 w-8" @if (2*s+2)<=rating {" text-yellow-400"} @else {" text-zinc-700 group-hover:text-yellow-400"}} {
                                (svg::star_right())
                            }
                        }
                    }
                } @else {
                    div class="relative z-0 flex flex-row text-zinc-700 size-fit" {
                        div class="absolute left-1/2 top-1/2 translate-x-[-50%] translate-y-[-50%] text-white select-none" {
                            "Login to rate item"
                        }
                        @for _ in 0..5 {
                            div class="w-8"{
                                (svg::star_left())
                            }
                            div class="w-8"{
                                (svg::star_right())
                            }
                        }
                    }
                }
                br;
                b {"Description"}
                br;
                div class="whitespace-pre-line"{
                    (item.description)
                }
            }
        }
        div class="mt-4 text-white" {
            div class="mx-auto flex flex-col text-white w-full gap-4 max-w-[39rem]" {
                b {"User ratings"}
                @if let Some(page) = page
                {
                    @for rating in &page.items {
                        a href={"/users/" (rating.user.username) } hx-boost="true" hx-target="#content" {
                            div class="p-4 h-20 w-full flex flex-row items-center bg-zinc-900 rounded-md" {
                                div class="basis-1/3 flex flex-col items-center" {
                                    @if rating.user.has_avatar {
                                            div style={"background-image:url('/static/images/avatars/" (rating.user.username) "')"} class="bg-cover bg-center size-8 rounded-full overflow-hidden" {}

                                    } @else {
                                        div style={"background-color:hsl(" (rating.user.avatar_hue) ",100%,50%)"} class="grid justify-center content-center size-8 text-white rounded-full" {
                                            div class="size-6" {
                                                (svg::user())
                                            }
                                        }
                                    }
                                    b {
                                        (rating.user.username)
                                    }
                                    @if rating.user.is_admin {
                                        span class="bg-violet-400 text-white px-2 text-xs" {
                                                "admin"
                                        }
                                    }
                                }
                                div class="basis-1/3 flex flex-row size-fit justify-center" {
                                    @for s in 0..5 {
                                        div class={"w-6" @if (2*s+1)<=rating.rating {" text-yellow-400"} @else {" text-zinc-700"}} {
                                            (svg::star_left())
                                        }
                                        div class={"w-6" @if (2*s+2)<=rating.rating {" text-yellow-400"} @else {" text-zinc-700"}} {
                                            (svg::star_right())
                                        }
                                    }
                                }
                                div class="basis-1/3 text-center" {
                                    (rating.date.format("%b %d, %Y"))
                                }
                            }
                        }
                    }
                    @for _ in 0..3usize.checked_sub(page.items.len()).unwrap_or_default() {
                        div class="grid justify-center content-center bg-zinc-700 rounded-md h-20 w-full max-w-[39rem] p-4" {}
                    }
                (pagination(page))
                } @else {
                    div class="grid justify-center content-center bg-zinc-700 rounded-md h-20 w-full max-w-[39rem] p-4" {
                        "No user ratings for this item!"
                    }
                }

            }
        }
    }
}

pub fn item_view(
    page_opt: Option<database::Page<database::Item>>,
    user: Option<&database::User>,
) -> Markup {
    html! {
        @if let Some(user) = user {
            @if user.is_admin {
                div class="mb-4 flex flex-row flex-wrap gap-x-4 justify-center" {
                    div class="w-56"{
                        button hx-get="/items/add" hx-swap="afterend" class="rounded-full p-2 bg-violet-400 hover:bg-black hover:text-white" {
                            "Add item"
                        }
                    }
                    div class="w-56 h-0"{}
                    div class="w-56 h-0"{}
                    div class="w-56 h-0"{}
                }
            }
        }
        @if let Some(page) = page_opt {
            div class="flex flex-row flex-wrap gap-4 justify-center" {
                @for item in &page.items {
                    a href={"/items/" (item.locator)} hx-boost="true" hx-target="#content" {
                        div class="group relative z-0 w-56 aspect-[3/4] rounded-md overflow-hidden outline outline-offset-2 outline-2 outline-transparent hover:outline-violet-400" {
                            div style={"background-image: url('/static/images/items/" (item.locator) "')"} class="size-full bg-cover bg-center group-hover:brightness-75 transition-[filter]" {}
                            div class="absolute w-full h-24 top-0 bg-gradient-to-b from-black to-transparent" {
                                div class="m-2 text-white text-xs flex flex-col items-center size-fit" {
                                    div class="text-yellow-400 flex flex-row w-8" {
                                        (svg::star_left())
                                        (svg::star_right())
                                    }
                                    div {
                                        (format!("{:.2}",item.score))
                                    }
                                }
                            }
                            div class="absolute w-full h-24 bottom-0 text-white text-center bg-gradient-to-t from-black to-transparent flex flex-col justify-end p-4" {
                                (item.title)
                            }
                        }
                    }
                }
                @for _ in 0..12usize.checked_sub(page.items.len()).unwrap_or_default() {
                    div class="w-56 aspect-[3/4] bg-zinc-700 rounded-md" {}
                }
            }
            (pagination(page))
        } @else {
            div class="mx-auto text-white grid justify-center content-center bg-zinc-700 rounded-md h-20 w-full max-w-[39rem] p-4" {
                "No matching entries found!"
            }
        }
    }
}

pub fn user_view(page_opt: Option<database::Page<database::User>>) -> Markup {
    if let Some(page) = page_opt {
        html! {
            div class="flex flex-row flex-wrap gap-4 justify-center" {
                @for item in &page.items {
                    a href={"/users/" (item.username)} hx-boost="true" hx-target="#content" {
                        div class="group w-56 aspect-[3/4] grid justify-center content-center" {
                            div class="flex flex-col justify-between content-center text-white" {
                                @if item.has_avatar
                                {
                                    div style={"background-image:url('/static/images/avatars/" (item.username) "')"} class="bg-cover bg-center size-56 rounded-full group-hover:brightness-75 transition-[filter] overflow-hidden outline outline-offset-2 outline-2 outline-transparent group-hover:outline-violet-400" {}
                                } @else {
                                    div style={"background-color:hsl(" (item.avatar_hue) ",100%,50%)"} class="relative z-0 size-56 grid justify-center content-center rounded-full group-hover:brightness-75 transition-[filter] overflow-hidden outline outline-offset-2 outline-2 outline-transparent group-hover:outline-violet-400" {
                                        div class="size-[10.5rem]"{
                                            (svg::user())
                                        }
                                    }
                                }
                                div class="flex flex-row justify-center items-center pt-4"
                                {
                                    (item.username)
                                    @if item.is_admin {
                                        span class="bg-violet-400 text-white px-2 text-xs" {
                                            b {
                                                "admin"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                @for _ in 0..12usize.checked_sub(page.items.len()).unwrap_or_default() {
                    div class="w-56 aspect-[3/4] grid justify-center content-center" {
                        div class="flex flex-col justify-between content-center text-white" {
                            div class="size-56 bg-zinc-700 rounded-full" {}
                            div class="min-h-10" {}
                        }
                    }

                }
            }
            (pagination(page))
        }
    } else {
        html! {
            div class="mx-auto text-white grid justify-center content-center bg-zinc-700 rounded-md h-20 w-full max-w-[39rem] p-4" {
                "No matching entries found!"
            }
        }
    }
}

pub fn user_page(
    page_user: &database::User,
    page: Option<database::Page<database::RatingUser>>,
    user: Option<&database::User>,
) -> Markup {
    html! {
        @if let Some(user) = user {
            @if user.username == page_user.username || user.is_admin {
                div class="mb-4 flex flex-row gap-x-4" {
                    button hx-get={"/users/" (page_user.username) "/edit"} hx-swap="afterend" class="rounded-full p-2 bg-violet-400 hover:bg-black hover:text-white" {
                        "Edit user"
                    }
                    @if !page_user.is_admin {
                        button hx-get={"/users/" (page_user.username) "/remove"} hx-swap="afterend"  class="rounded-full p-2 bg-violet-400 hover:bg-black hover:text-white" {
                            "Remove user"
                        }
                    }
                }
            }
        }
        div class="flex flex-col gap-4 content-center items-center" {
            div {
                @if page_user.has_avatar {
                    div style={"background-image:url('/static/images/avatars/" (page_user.username) "')"} class="bg-cover bg-center size-64 rounded-full overflow-hidden" {}
                } @else {
                    div style={"background-color:hsl(" (page_user.avatar_hue) ",100%,50%)"} class="text-white size-64 grid justify-center content-center rounded-full overflow-hidden" {
                        div class="size-[12rem]"{
                            (svg::user())
                        }
                    }
                }
            }
            div class="text-white" {
                div class="flex flex-row items-center" {
                    b class="text-2xl" {
                        (page_user.username)
                    }
                    @if page_user.is_admin {
                        b class="bg-violet-400 px-4 text-lg" {
                            "admin"
                        }
                    }
                }
            }
            div class="mx-auto flex flex-col text-white w-full gap-4 max-w-[39rem]" {
                b {"User ratings"}
                @if let Some(page) = page
                {
                    @for rating in &page.items {
                        a href={"/items/" (rating.item.locator) } hx-boost="true" hx-target="#content" {
                            div class="w-full p-4 h-20 flex flex-row items-center bg-zinc-900 rounded-md" {
                                div class="basis-1/3 flex flex-row items-center" {
                                    b class="text-xs" {
                                        (rating.item.title)
                                    }
                                }
                                div class="basis-1/3 flex flex-row size-fit justify-center" {
                                    @for s in 0..5 {
                                        div class={"w-6" @if (2*s+1)<=rating.rating {" text-yellow-400"} @else {" text-zinc-700"}} {
                                            (svg::star_left())
                                        }
                                        div class={"w-6" @if (2*s+2)<=rating.rating {" text-yellow-400"} @else {" text-zinc-700"}} {
                                            (svg::star_right())
                                        }
                                    }
                                }
                                div class="basis-1/3 text-center" {
                                    (rating.date.format("%b %d, %Y"))
                                }
                            }
                        }
                    }
                    @for _ in 0..3usize.checked_sub(page.items.len()).unwrap_or_default() {
                        div class="grid justify-center content-center bg-zinc-700 rounded-md h-20 w-full max-w-[39rem] p-4" {}
                    }
                (pagination(page))
                } @else {
                    div class="grid justify-center content-center bg-zinc-700 rounded-md h-20 w-full max-w-[39rem] p-4" {
                        "User has no reviews!"
                    }
                }

            }
        }
    }
}

pub fn logged_in(user: &database::User) -> Markup {
    html! {
        div class="select-none relative z-10 group flex flex-row items-center bg-white rounded-[1rem] hover:rounded-b-none" {
            div class="ms-2" {
                (user.username)
            }
            @if user.is_admin {
                div class="bg-violet-400 text-white px-2 text-xs" {
                    b {
                        "admin"
                    }
                }
            }
            @if user.has_avatar {
                    div style={"background-image:url('/static/images/avatars/" (user.username) "')"} class="ms-2 bg-cover bg-center size-8 rounded-full overflow-hidden" {}

            } @else {
                div style={"background-color:hsl(" (user.avatar_hue) ",100%,50%)"} class="ms-2 grid justify-center content-center size-8 text-white rounded-full" {
                    div class="size-6" {
                        (svg::user())
                    }
                }
            }
            div class="absolute top-8 w-full hidden group-hover:block" {
                div class="flex flex-col justify-center bg-white rounded-b-[1rem]" {
                    a href={"/users/" (user.username)} hx-boost="true" hx-target="#content" class="text-center rounded-full h-8 grid justify-content content-center hover:bg-black hover:text-white" {
                        "Profile"
                    }
                    button hx-post="/logout" class="rounded-full h-8 hover:bg-black hover:text-white" {
                        "Logout"
                    }
                }
            }
        }
    }
}

pub fn login_button() -> Markup {
    html! {
        button hx-get="/login" class="bg-white rounded-full px-4 h-8 hover:bg-black hover:text-white" {
            "Login"
        }
    }
}

pub fn remove_form(endpoint: &str, button_prompt: &str, item: &str) -> Markup {
    html! {
        div hx-target="this" class="fixed left-0 top-0 w-full h-full flex justify-center z-50" {
            div _="on click remove closest parent <div/>" class="absolute w-full h-full bg-black/50" {}
            form hx-post=(endpoint) hx-swap="outerHTML" class="flex flex-col gap-4 absolute bg-zinc-800 p-4 rounded-md top-1/4 w-96" {
                div class="text-white text-center" {
                    "Are you absolutely sure that you want to remove " span class="text-violet-400" {(item)} "? This operation is irreversible."
                }
                button class="h-8 bg-violet-400 rounded-full hover:bg-black hover:text-white" type="submit" {(button_prompt)}
            }
        }
    }
}

pub fn user_edit_form(message: Option<&str>, username: &str) -> Markup {
    html! {
        div hx-target="this" class="fixed left-0 top-0 w-full h-full flex justify-center z-50" {
            div _="on click remove closest parent <div/>" class="absolute w-full h-full bg-black/50" {}
            form hx-post={"/users/" (username) "/edit"} hx-swap="outerHTML" class="flex flex-col gap-4 absolute bg-zinc-800 p-4 rounded-md top-1/4 w-96" enctype="multipart/form-data" {
                @if let Some(message)=message
                {
                    div class="grid justify-center content-center px-2 min-h-8 text-center bg-orange-200 text-orange-400 rounded-[1rem]" {
                        (message)
                    }
                }
                div {
                    label for="username" class="block mb-2 text-sm text-violet-400" {"Username"}
                    input class="p-2 w-full h-8 rounded-full text-center text-black bg-white outline outline-offset-2 outline-2 outline-transparent focus:outline-violet-400" type="text" name="username" id="username" value=(username) hx-preserve;
                }
                div {
                    label for="password1" class="block mb-2 text-sm text-violet-400" {"New password"}
                    input class="p-2 w-full h-8 rounded-full text-center text-black bg-white outline outline-offset-2 outline-2 outline-transparent focus:outline-violet-400" type="password" name="password1" id="password1" hx-preserve;
                }
                div {
                    label for="password2" class="block mb-2 text-sm text-violet-400" {"Repeat new password"}
                    input class="p-2 w-full h-8 rounded-full text-center text-black bg-white outline outline-offset-2 outline-2 outline-transparent focus:outline-violet-400" type="password" name="password2" id="password2" hx-preserve;
                }
                div class="group" {
                    label for="avatar" class="block mb-2 text-sm text-violet-400" {"Avatar"}
                    input class="w-full h-8 rounded-full text-center text-black bg-white outline outline-offset-2 outline-2 outline-transparent focus:outline-violet-400 file:bg-violet-400 file:rounded-full file:border-none file:h-full justify-center content-center group-hover:file:text-white group-hover:file:bg-black" type="file" name="avatar" id="avatar" accept="image/*" hx-preserve;
                }
                div {
                    label for="clear_avatar" class="block mb-2 text-sm text-violet-400" {"Clear avatar"}
                    input class="size-8 rounded-full accent-violet-400 checked:hover:accent-black" type="checkbox" name="clear_avatar" id="clear_avatar" hx-preserve;
                }
                button class="h-8 bg-violet-400 rounded-full hover:bg-black hover:text-white" type="submit" {"Edit user"}
            }
        }
    }
}

pub fn item_form(
    endpoint: &str,
    button_prompt: &str,
    message: Option<&str>,
    title: Option<&str>,
    locator: Option<&str>,
    description: Option<&str>,
) -> Markup {
    html! {
        div hx-target="this" class="fixed left-0 top-0 w-full h-full flex justify-center z-50" {
            div _="on click remove closest parent <div/>" class="absolute w-full h-full bg-black/50" {}
            form hx-post=(endpoint) hx-swap="outerHTML" class="flex flex-col gap-4 absolute bg-zinc-800 p-4 rounded-md top-1/4 w-96" enctype="multipart/form-data" {
                @if let Some(message)=message
                {
                    div class="grid justify-center content-center px-2 min-h-8 text-center bg-orange-200 text-orange-400 rounded-[1rem]" {
                        (message)
                    }
                }
                div {
                    label for="title" class="block mb-2 text-sm text-violet-400" {"Title"}
                    input class="p-2 w-full h-8 rounded-full text-center text-black bg-white outline outline-offset-2 outline-2 outline-transparent focus:outline-violet-400" type="text" name="title" id="title" value=[title] hx-preserve;
                }
                div {
                    label for="locator" class="block mb-2 text-sm text-violet-400" {"Locator"}
                    input class="p-2 w-full h-8 rounded-full text-center text-black bg-white outline outline-offset-2 outline-2 outline-transparent focus:outline-violet-400" type="text" name="locator" id="locator" value=[locator] hx-preserve;
                }
                div {
                    label for="description" class="block mb-2 text-sm text-violet-400" {"Description"}
                    textarea style="scrollbar-width: none" class="p-2 w-full min-h-32 rounded-[1rem] text-black bg-white outline outline-offset-2 outline-2 outline-transparent focus:outline-violet-400" name="description" id="description" hx-preserve {
                        @if let Some(description) = description {
                            (description)
                        }
                    }
                }
                div class="group" {
                    label for="image" class="block mb-2 text-sm text-violet-400" {"Cover image"}
                    input class="w-full h-8 rounded-full text-center text-black bg-white outline outline-offset-2 outline-2 outline-transparent focus:outline-violet-400 file:bg-violet-400 file:rounded-full file:border-none file:h-full justify-center content-center group-hover:file:text-white group-hover:file:bg-black" type="file" name="image" id="image" accept="image/*" hx-preserve;
                }
                button class="h-8 bg-violet-400 rounded-full hover:bg-black hover:text-white" type="submit" {(button_prompt)}
            }
        }
    }
}

pub fn login_form(message: Option<&str>) -> Markup {
    html! {
        (login_button())
        div class="fixed left-0 top-0 w-full h-full flex justify-center z-50" {
            div _="on click remove closest parent <div/>" class="absolute w-full h-full bg-black/50" {}
            form hx-post="/login" class="flex flex-col gap-4 absolute bg-zinc-800 p-4 rounded-md top-1/4 w-96" {
                @if let Some(message)=message
                {
                    div class="grid justify-center content-center px-2 min-h-8 text-center bg-orange-200 text-orange-400 rounded-[1rem]" {
                        (message)
                    }
                }
                div {
                    label for="username" class="block mb-2 text-sm text-violet-400" {"Username"}
                    input class="p-2 w-full h-8 rounded-full text-center text-black bg-white outline outline-offset-2 outline-2 outline-transparent focus:outline-violet-400" type="text" name="username" id="username" hx-preserve;
                }
                div {
                    label for="password" class="block mb-2 text-sm text-violet-400" {"Password"}
                    input class="p-2 w-full h-8 rounded-full text-center text-black bg-white outline outline-offset-2 outline-2 outline-transparent focus:outline-violet-400" type="password" name="password" id="password" hx-preserve;
                }
                button class="h-8 bg-violet-400 rounded-full hover:bg-black hover:text-white transition-colors" type="submit" {"Login"}
                button hx-get="/register" class="h-8 bg-white rounded-full hover:bg-black hover:text-white" {"Register"}
            }
        }
    }
}

pub fn register_form(message: Option<&str>) -> Markup {
    html! {
        (login_button())
        div class="fixed left-0 top-0 w-full h-full flex justify-center z-50" {
            div _="on click remove closest parent <div/>" class="absolute w-full h-full bg-black/50" {}
            form hx-post="/register" class="flex flex-col gap-4 absolute bg-zinc-800 p-4 rounded-md top-1/4 w-96" {
                @if let Some(message)=message
                {
                    div class="grid justify-center content-center px-2 min-h-8 text-center bg-orange-200 text-orange-400 rounded-[1rem]" {
                        (message)
                    }
                }
                div {
                    label for="username" class="block mb-2 text-sm text-violet-400" {"Username"}
                    input class="p-2 w-full h-8 rounded-full text-center text-black bg-white outline outline-offset-2 outline-2 outline-transparent focus:outline-violet-400" type="text" name="username" id="username" hx-preserve;
                }
                div {
                    label for="password1" class="block mb-2 text-sm text-violet-400" {"Password"}
                    input class="p-2 w-full h-8 rounded-full text-center text-black bg-white outline outline-offset-2 outline-2 outline-transparent focus:outline-violet-400" type="password" name="password1" id="password1" hx-preserve;
                }
                div {
                    label for="password2" class="block mb-2 text-sm text-violet-400" {"Repeat password"}
                    input class="p-2 w-full h-8 rounded-full text-center text-black bg-white outline outline-offset-2 outline-2 outline-transparent focus:outline-violet-400" type="password" name="password2" id="password2" hx-preserve;
                }
                button class="h-8 bg-violet-400 rounded-full hover:bg-black hover:text-white transition-colors" type="submit" {"Register"}
                button hx-get="/login" class="h-8 bg-white rounded-full hover:bg-black hover:text-white transition-colors" {"Login"}
            }
        }
    }
}

pub fn search(target: &str, content: Option<Markup>) -> Markup {
    html! {
        form action=(target) method="get" hx-boost="true" hx-target="#content" hx-trigger="input changed from:input delay:500ms" class="absolute w-full" {
            input autofocus type="text" placeholder="Search" name="search" class="appearance-none w-full h-8 text-center text-black bg-white outline outline-offset-2 outline-2 outline-transparent focus:outline-white rounded-full" {}
        }
        div class="absolute right-0 z-10" {
            div class="relative group grid justify-content content-center bg-white px-4 h-8 rounded-[1rem] hover:rounded-b-none select-none" {
                @if target=="/items" {
                    "Items"
                } @else if target=="/users" {
                    "Users"
                }
                div class="absolute top-8 w-full hidden group-hover:block" {
                    div class="flex flex-col justify-center bg-white rounded-b-[1rem]" {
                        @if target=="/items" {
                            button hx-get="/search?target=users" class="rounded-full h-8 hover:bg-black hover:text-white" {
                                "Users"
                            }
                        } @else if target=="/users" {
                            button hx-get="/search?target=items" class="rounded-full h-8 hover:bg-black hover:text-white" {
                                "Items"
                            }
                        }
                    }
                }
            }
        }
        @if let Some(content) = content {
            div id="content" hx-swap-oob="innerHTML" {
                (content)
            }
        }
    }
}

pub fn index(content: Markup, search_target: &str, user: Option<&database::User>) -> Markup {
    html! {
        (DOCTYPE)
        html {
            head {
                title {
                    "Title"
                }
                meta charset="UTF-8";
                meta name="author" content="Jakub Grodzki 240675";
                meta name="viewport" content="width=device-width, initial-scale=1.0";
                meta name="htmx-config" content="{\"scrollIntoViewOnBoost\":false}";
                script src="https://unpkg.com/htmx.org@1.9.11" {}
                script src="https://unpkg.com/hyperscript.org@0.9.12" {}
                link rel="stylesheet" href="/static/style.css";
                link rel="icon" href="/static/icon.png";
                link rel="preconnect" href="https://fonts.googleapis.com";
                link rel="preconnect" href="https://fonts.gstatic.com" crossorigin;
                link href="https://fonts.googleapis.com/css2?family=Quicksand:wght@500&display=swap" rel="stylesheet";

            }
            body class="flex flex-col bg-zinc-900 min-h-screen min-w-[31rem] font-[Quicksand]" {
                header class="top-0 sticky z-40 flex justify-between items-center bg-violet-400 text-black mx-auto w-full max-w-screen-lg p-4" {
                    div class="flex h-8 justify-start basis-1/4" {
                        a href="/" hx-boost="true" hx-target="#content" {
                            (svg::logo())
                        }
                    }
                    div class="relative z-10 h-8 rounded-full w-1/2 flex flex-row mx-4" hx-target="this" {
                        (search(search_target, None))
                    }
                    div hx-target="this" class="flex justify-end basis-1/4" {
                        @if let Some(user) = user {
                            (logged_in(user))
                        }
                        @else
                        {
                            (login_button())
                        }
                    }
                }
                div id="content" class="min-h-full flex-1 bg-zinc-800 mx-auto w-full max-w-screen-lg p-4" {
                    (content)
                }
            }
        }
    }
}

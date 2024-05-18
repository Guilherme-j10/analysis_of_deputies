use anyhow::Result;
use reqwest;
use scraper::{selectable::Selectable, Html, Selector};

#[derive(Debug)]
struct ParliamentaryLink {
    name: String,
    link: String,
    tag: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut deputies_list: Vec<ParliamentaryLink> = Vec::new();
    let mut page: u8 = 1;
    let mut still_get_link: bool = true;

    while still_get_link == true {
        let format_url = format!("https://www.camara.leg.br/deputados/quem-sao/resultado?search=&partido=&uf=&legislatura=&sexo=&pagina={}", page);
        println!("scraping page -> {:?}", format_url);
        let scrapad_page = scrape_list_page(&format_url).await;

        if scrapad_page.len() == 0 {
            still_get_link = false;
        }

        deputies_list.extend(scrapad_page);
        page += 1;
    }

    for deputie in deputies_list {
        let date = date_of_birth(&deputie.link).await;
        println!("{}", date);
    }

    Ok(())
}

async fn date_of_birth(url: &str) -> String {
    let response_html = reqwest::get(url).await.unwrap().text().await.unwrap();
    let document = Html::parse_document(&response_html);
    let mut date = "";

    let selector = Selector::parse("ul.informacoes-deputado").unwrap();
    let li_selector = Selector::parse("li").unwrap();

    let lis = document
        .select(&selector)
        .next()
        .unwrap()
        .select(&li_selector);

    for item in lis {
        let item_content = item.text().collect::<Vec<&str>>();

        if item_content.get(0).unwrap().contains("Data") {
            date = item_content.get(1).unwrap().trim();
        }
    }

    date.to_owned()
}

async fn scrape_list_page(url: &str) -> Vec<ParliamentaryLink> {
    let avaliable_tags: [&str; 2] = ["Em exercício", "Licenciado"];
    let html_document = reqwest::get(url).await.unwrap().text().await.unwrap();
    let mut scraped_data: Vec<ParliamentaryLink> = Vec::new();
    let document = Html::parse_document(&html_document);
    let selector_names = Selector::parse("h3.lista-resultados__cabecalho").unwrap();

    document.select(&selector_names).for_each(|element| {
        let selector_link = Selector::parse("a").unwrap();
        let selector_span_tag = Selector::parse("span").unwrap();

        let tag_value = element
            .select(&selector_span_tag)
            .next()
            .unwrap()
            .inner_html();

        if avaliable_tags.contains(&tag_value.as_str()) {
            let a_tag = element.select(&selector_link).next().unwrap();

            scraped_data.push(ParliamentaryLink {
                name: a_tag.inner_html().to_owned(),
                link: a_tag.value().attr("href").unwrap().to_owned(),
                tag: tag_value.to_owned(),
            });
        }
    });

    scraped_data
}

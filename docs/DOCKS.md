## POST REQUEST /generate_file

```
{
    "provider_id": String | null,
    "merchant_id": String | null
    "filters": [
        {
            "id": Number,
            "status": String | null,
            "mode": String | null,
            "payments_system": [String... | empty] | null,
        },
        "other filters..."
    ],
    "report_type": String | null,
    "monthly_subscription_fee": Floor | null
}

```

Описание каждого входящего параметра в запросе
1. `provider_id` - это уникальный id провайдер, по указанному значению будет искаться нужный нам провайдер.
используется в случае если наш `report_type` указан как `Agent` или `TaxiCompany` в случае если `report_type` указан как `Merchant`, в `provider_id` должен находится `null`
2. `merchant_id` - это уникальный id вендора, по указанному значению будет искаться нужный нам вендор
используется в случае если наш `report_type` указан как `Merchant` в случае если `report_type` указан как `Agent` или `TaxiCompany`, в merchant_id должен находится `null`
3. `filters` - это массив фильтров, в каждом переданном фильтре находятся настройки под каждый файл по которому будет идти процесс генерации.
    - [ ] `filter` - принимает в себя такие поля как:
    -  `id` - id сгенерированного файла
   `status` - status это status транзакций который нам нужен, есть несколько видов статусов 
        - 1 - Completed(Завершена)
        - 2 - Mistake(Ошибка)
        - 3 - Created(Создана)
        - 4 - Cancel(Отмена)
        - 5 - Unknown не должен передаваться в аргументах, если статус Unknown, то вы получите ошибку
   - `mode` mode - это режим транзакций, бывает несколько видов транзакций к примеру такие как Боевой или Тестовый
   - `payments_system` - payments_system это массив который фильтрует транзакции по платёжным системам, к примеру мы можем указать две платежные системы ["payment_system_name", "payment_system_name"]
   в таком случае мы будем суммировать транзакции из файла только с текущими платежными системамиы.
4. `report_type` - report_type это тип отчета, есть 3 вида типов отчета
    - 1 Agent - отчет по агентам 
      - [ ] Agent должен принимать в filters обязательно id отчета который имеет тип (pay, pay_f)
          так-же Agent может принять как дополнительный фильтр (c2card, c2cCOMANYNAME). 
    - 2 TaxiCompany - отчет по таксопаркам.
        - [ ] TaxiCompany должен принимать в filters обязательно id отчета который имеет тип (pay, pay_f)
        так-же TaxiCompany может принять как дополнительный фильтр (c2card, c2cCOMANYNAME). 
    - 3 Merchant - отчет по мерчантам
        - [ ] Merchant принимает в filters обязателньй id отчета который имеет тип (pay, pay_f).  
5. `monthly_subscription_fee` - monthly_subscription_fee это абонентская плата таксопарка, текущее поле нужно в случае если `report_type` был указан `TaxiCompany`
пользователь может не передавать сумму абонентской платы, в таком случае подставится дефолтное число  `1.000.000`, но если у таксопарка другая сумма абоненской платы, то пользователь может указать ее в этом поле.

В ответ на успешный запрос вы получить подобный json ответ

```
{
    "error": null,
    "result": {
        "path": "/reports/file_name.xlsx"
    }
}
```
Поле `path` содержит в себе путь до файла

В ответ на не успешный запрос вы получить подобный json ответ
пример:
```
{
    "error": {,
        "code": 4334304,
        "message": "Файл под id *** не содержит в себе нужных вам данных"
    }
    "result": null
}
```

## GET REQUEST /get_share
```
{
    "error": null,
    "result": {
        "share": {
            "generated_now": 0,
            "max_count_record_in_reports": 1000,
            "reports": {
                "data": {}
            }
        }
    }
}
```

1. `generated_now` - generated_now в этом поле написано число генерируемых отчетов в текущий момент
2. `max_count_record_in_reports` - max_count_record_in_reports содержит в себе максимальное число отчетов которые могут содержаться в поле `reports.data`
3. `reports` - reports содержит в себе поле `data` в котором хранятся сгенерированные отчеты.
То-есть reports в себя кеширует данные отчета, который был сгенерирован, но эти данные временные, переодическеий в reports удаляются старые данные.

`share` может сильно разростись, но метода который получает конкретный `report` пока не существует на момент 4 сентября 2023.

## GET REQUEST /download/{path}

`/download/{file_name}` Метод рабочий но не рекомендуется его использовать, `{file_name}` это имя файла который мы хотим скачать.
метод не чего не возвращает, вы просто скачиваете файл.

## GET REQUEST /download/get_weight/{file_name} 

`/download/get_weight/{file_name}` Возвращает размер файла
```
{
    "error": null,
    "result": {
        "bytes": Number
    }
}
```
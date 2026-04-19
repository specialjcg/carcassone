module Main exposing (main)

import Browser
import Html exposing (Html, div, h1, text)


main : Program () () ()
main =
    Browser.sandbox
        { init = ()
        , update = \_ _ -> ()
        , view = view
        }


view : () -> Html ()
view _ =
    div []
        [ h1 [] [ text "Carcassonne" ]
        , Html.p [] [ text "scaffold, game logic to come" ]
        ]

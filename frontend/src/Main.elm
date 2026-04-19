module Main exposing (main)

import Browser
import Html exposing (Html, button, div, h1, h2, li, p, text, ul)
import Html.Attributes exposing (class, disabled, style)
import Html.Events exposing (onClick)
import Http
import Json.Decode as D exposing (Decoder)
import Json.Encode as E



-- API


apiBase : String
apiBase =
    "http://localhost:3000"



-- TYPES


type alias Pos =
    ( Int, Int )


type Side
    = North
    | East
    | South
    | West


type EdgeKind
    = Road
    | City
    | Field


type alias TileSpec =
    { edges : List EdgeKind
    , segments : List Int
    , monastery : Bool
    , shield : Bool
    }


type alias PlacedTile =
    { spec : TileSpec
    , rotation : Int
    }


type MeepleChoice
    = OnSegment Side
    | OnMonastery


type alias GreedyMove =
    { pos : Pos
    , rotation : Int
    , meeple : Maybe MeepleChoice
    }


type alias ActiveMeeple =
    { pos : Pos
    , on : MeepleChoice
    , owner : Int
    }


type alias CellView =
    { pos : Pos
    , tile : PlacedTile
    }


type alias BoardView =
    { cells : List CellView
    , meeples : List ActiveMeeple
    , lastPlaced : Maybe Pos
    }


type alias Player =
    { id : Int
    , score : Int
    , meeplesRemaining : Int
    }


type FeatureKind
    = FRoad
    | FCity
    | FMonastery
    | FFarm


type alias ScoringEvent =
    { kind : FeatureKind
    , points : Int
    , winners : List Int
    , meeplesReturned : List Int
    }


type alias GameView =
    { board : BoardView
    , players : List Player
    , bagRemaining : Int
    , currentPlayer : Int
    , currentDraw : Maybe TileSpec
    , isOver : Bool
    , finished : Bool
    }


type alias CreateGameResponse =
    { gameId : String
    , state : GameView
    }


type alias TurnResponse =
    { state : GameView
    , events : List ScoringEvent
    , finished : Bool
    }



-- DECODERS


decodePos : Decoder Pos
decodePos =
    D.map2 Tuple.pair
        (D.index 0 D.int)
        (D.index 1 D.int)


decodeSide : Decoder Side
decodeSide =
    D.string
        |> D.andThen
            (\s ->
                case s of
                    "North" ->
                        D.succeed North

                    "East" ->
                        D.succeed East

                    "South" ->
                        D.succeed South

                    "West" ->
                        D.succeed West

                    other ->
                        D.fail ("unknown side: " ++ other)
            )


decodeEdgeKind : Decoder EdgeKind
decodeEdgeKind =
    D.string
        |> D.andThen
            (\s ->
                case s of
                    "Road" ->
                        D.succeed Road

                    "City" ->
                        D.succeed City

                    "Field" ->
                        D.succeed Field

                    other ->
                        D.fail ("unknown edge: " ++ other)
            )


decodeTileSpec : Decoder TileSpec
decodeTileSpec =
    D.map4 TileSpec
        (D.field "edges" (D.list decodeEdgeKind))
        (D.field "segments" (D.list D.int))
        (D.field "monastery" D.bool)
        (D.field "shield" D.bool)


decodePlacedTile : Decoder PlacedTile
decodePlacedTile =
    D.map2 PlacedTile
        (D.field "spec" decodeTileSpec)
        (D.field "rotation" D.int)


decodeMeepleChoice : Decoder MeepleChoice
decodeMeepleChoice =
    D.oneOf
        [ D.string
            |> D.andThen
                (\s ->
                    if s == "Monastery" then
                        D.succeed OnMonastery

                    else
                        D.fail ("unexpected meeple tag: " ++ s)
                )
        , D.field "Segment" decodeSide |> D.map OnSegment
        ]


decodeGreedyMove : Decoder GreedyMove
decodeGreedyMove =
    D.map3 GreedyMove
        (D.field "pos" decodePos)
        (D.field "rotation" D.int)
        (D.field "meeple" (D.nullable decodeMeepleChoice))


decodeActiveMeeple : Decoder ActiveMeeple
decodeActiveMeeple =
    D.map3 ActiveMeeple
        (D.field "pos" decodePos)
        (D.field "on" decodeMeepleChoice)
        (D.field "owner" D.int)


decodeCellView : Decoder CellView
decodeCellView =
    D.map2 CellView
        (D.field "pos" decodePos)
        (D.field "tile" decodePlacedTile)


decodeBoardView : Decoder BoardView
decodeBoardView =
    D.map3 BoardView
        (D.field "cells" (D.list decodeCellView))
        (D.field "meeples" (D.list decodeActiveMeeple))
        (D.field "last_placed" (D.nullable decodePos))


decodePlayer : Decoder Player
decodePlayer =
    D.map3 Player
        (D.field "id" D.int)
        (D.field "score" D.int)
        (D.field "meeples_remaining" D.int)


decodeFeatureKind : Decoder FeatureKind
decodeFeatureKind =
    D.string
        |> D.andThen
            (\s ->
                case s of
                    "Road" ->
                        D.succeed FRoad

                    "City" ->
                        D.succeed FCity

                    "Monastery" ->
                        D.succeed FMonastery

                    "Farm" ->
                        D.succeed FFarm

                    other ->
                        D.fail ("unknown feature kind: " ++ other)
            )


decodeScoringEvent : Decoder ScoringEvent
decodeScoringEvent =
    D.map4 ScoringEvent
        (D.field "kind" decodeFeatureKind)
        (D.field "points" D.int)
        (D.field "winners" (D.list D.int))
        (D.field "meeples_returned" (D.list D.int))


decodeGameView : Decoder GameView
decodeGameView =
    D.map7 GameView
        (D.field "board" decodeBoardView)
        (D.field "players" (D.list decodePlayer))
        (D.field "bag_remaining" D.int)
        (D.field "current_player" D.int)
        (D.field "current_draw" (D.nullable decodeTileSpec))
        (D.field "is_over" D.bool)
        (D.field "finished" D.bool)


decodeCreateGameResponse : Decoder CreateGameResponse
decodeCreateGameResponse =
    D.map2 CreateGameResponse
        (D.field "game_id" D.string)
        (D.field "state" decodeGameView)


decodeTurnResponse : Decoder TurnResponse
decodeTurnResponse =
    D.map3 TurnResponse
        (D.field "state" decodeGameView)
        (D.field "events" (D.list decodeScoringEvent))
        (D.field "finished" D.bool)



-- ENCODERS


encodeSide : Side -> E.Value
encodeSide s =
    E.string
        (case s of
            North ->
                "North"

            East ->
                "East"

            South ->
                "South"

            West ->
                "West"
        )


encodeMeepleChoice : MeepleChoice -> E.Value
encodeMeepleChoice c =
    case c of
        OnMonastery ->
            E.string "Monastery"

        OnSegment s ->
            E.object [ ( "Segment", encodeSide s ) ]


encodePos : Pos -> E.Value
encodePos ( x, y ) =
    E.list E.int [ x, y ]


encodeGreedyMove : GreedyMove -> E.Value
encodeGreedyMove m =
    E.object
        [ ( "pos", encodePos m.pos )
        , ( "rotation", E.int m.rotation )
        , ( "meeple"
          , case m.meeple of
                Nothing ->
                    E.null

                Just c ->
                    encodeMeepleChoice c
          )
        ]



-- HTTP


createGame : Int -> Maybe Int -> Cmd Msg
createGame numPlayers maybeSeed =
    let
        body =
            E.object
                ([ ( "num_players", E.int numPlayers ) ]
                    ++ (case maybeSeed of
                            Just s ->
                                [ ( "seed", E.int s ) ]

                            Nothing ->
                                []
                       )
                )
    in
    Http.post
        { url = apiBase ++ "/games"
        , body = Http.jsonBody body
        , expect = Http.expectJson GotCreated decodeCreateGameResponse
        }


botTurn : String -> Cmd Msg
botTurn gameId =
    Http.post
        { url = apiBase ++ "/games/" ++ gameId ++ "/bot-turn"
        , body = Http.emptyBody
        , expect = Http.expectJson GotTurn decodeTurnResponse
        }


playMove : String -> GreedyMove -> Cmd Msg
playMove gameId mv =
    Http.post
        { url = apiBase ++ "/games/" ++ gameId ++ "/turn"
        , body = Http.jsonBody (encodeGreedyMove mv)
        , expect = Http.expectJson GotTurn decodeTurnResponse
        }


fetchLegalMoves : String -> Cmd Msg
fetchLegalMoves gameId =
    Http.get
        { url = apiBase ++ "/games/" ++ gameId ++ "/legal-moves"
        , expect = Http.expectJson GotLegalMoves (D.list decodeGreedyMove)
        }



-- MODEL


type Model
    = Loading
    | Playing GameState
    | Failed String


type alias GameState =
    { gameId : String
    , view : GameView
    , legalMoves : List GreedyMove
    , lastEvents : List ScoringEvent
    , busy : Bool
    }


init : () -> ( Model, Cmd Msg )
init _ =
    ( Loading, createGame 2 (Just 42) )



-- UPDATE


type Msg
    = GotCreated (Result Http.Error CreateGameResponse)
    | GotTurn (Result Http.Error TurnResponse)
    | GotLegalMoves (Result Http.Error (List GreedyMove))
    | ClickBotTurn
    | ClickPlayMove GreedyMove
    | ClickRestart


httpErrorToString : Http.Error -> String
httpErrorToString e =
    case e of
        Http.BadUrl s ->
            "bad url: " ++ s

        Http.Timeout ->
            "timeout"

        Http.NetworkError ->
            "network error (server down?)"

        Http.BadStatus s ->
            "HTTP " ++ String.fromInt s

        Http.BadBody msg ->
            "bad body: " ++ msg


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case ( msg, model ) of
        ( GotCreated (Ok resp), _ ) ->
            ( Playing
                { gameId = resp.gameId
                , view = resp.state
                , legalMoves = []
                , lastEvents = []
                , busy = False
                }
            , fetchLegalMoves resp.gameId
            )

        ( GotCreated (Err e), _ ) ->
            ( Failed (httpErrorToString e), Cmd.none )

        ( GotTurn (Ok resp), Playing gs ) ->
            ( Playing
                { gs
                    | view = resp.state
                    , lastEvents = resp.events
                    , busy = False
                }
            , if resp.finished then
                Cmd.none

              else
                fetchLegalMoves gs.gameId
            )

        ( GotTurn (Err e), Playing _ ) ->
            ( Failed (httpErrorToString e), Cmd.none )

        ( GotLegalMoves (Ok moves), Playing gs ) ->
            ( Playing { gs | legalMoves = moves }, Cmd.none )

        ( GotLegalMoves (Err _), Playing gs ) ->
            ( Playing { gs | legalMoves = [] }, Cmd.none )

        ( ClickBotTurn, Playing gs ) ->
            ( Playing { gs | busy = True }, botTurn gs.gameId )

        ( ClickPlayMove mv, Playing gs ) ->
            ( Playing { gs | busy = True }, playMove gs.gameId mv )

        ( ClickRestart, _ ) ->
            ( Loading, createGame 2 Nothing )

        _ ->
            ( model, Cmd.none )



-- VIEW


view : Model -> Html Msg
view model =
    div [ class "page" ]
        [ h1 [] [ text "Carcassonne" ]
        , case model of
            Loading ->
                p [] [ text "Loading…" ]

            Failed msg ->
                div []
                    [ p [ style "color" "red" ] [ text ("error: " ++ msg) ]
                    , button [ onClick ClickRestart ] [ text "Retry" ]
                    ]

            Playing gs ->
                viewGame gs
        ]


viewGame : GameState -> Html Msg
viewGame gs =
    let
        v =
            gs.view
    in
    div []
        [ viewHeader gs
        , viewPlayers v
        , viewCurrentDraw v
        , viewBoard v
        , viewActions gs
        , viewEvents gs.lastEvents
        , viewLegalMoves gs
        ]


viewHeader : GameState -> Html Msg
viewHeader gs =
    let
        v =
            gs.view

        status =
            if v.finished then
                "FINISHED"

            else
                "playing — bag " ++ String.fromInt v.bagRemaining
    in
    div [ class "header" ]
        [ p [] [ text ("game " ++ String.left 8 gs.gameId ++ "… — " ++ status) ]
        ]


viewPlayers : GameView -> Html Msg
viewPlayers v =
    div [ class "players" ]
        [ h2 [] [ text "Scores" ]
        , ul []
            (List.map
                (\p ->
                    let
                        marker =
                            if p.id == v.currentPlayer && not v.finished then
                                "▶ "

                            else
                                "  "
                    in
                    li []
                        [ text
                            (marker
                                ++ "Player "
                                ++ String.fromInt p.id
                                ++ ": "
                                ++ String.fromInt p.score
                                ++ " pts ("
                                ++ String.fromInt p.meeplesRemaining
                                ++ " meeples)"
                            )
                        ]
                )
                v.players
            )
        ]


viewCurrentDraw : GameView -> Html Msg
viewCurrentDraw v =
    case v.currentDraw of
        Nothing ->
            p [] [ text "No tile to draw." ]

        Just spec ->
            div [ class "draw" ]
                [ h2 [] [ text "Current draw" ]
                , p [] [ text (describeTile spec) ]
                ]


describeTile : TileSpec -> String
describeTile spec =
    let
        edges =
            spec.edges |> List.map edgeChar |> String.join ""

        flags =
            (if spec.monastery then
                " ⛪"

             else
                ""
            )
                ++ (if spec.shield then
                        " 🛡"

                    else
                        ""
                   )
    in
    "edges N E S W = [" ++ edges ++ "]" ++ flags


edgeChar : EdgeKind -> String
edgeChar e =
    case e of
        Road ->
            "R"

        City ->
            "C"

        Field ->
            "F"


viewBoard : GameView -> Html Msg
viewBoard v =
    div [ class "board-text" ]
        [ h2 [] [ text "Board" ]
        , p []
            [ text
                (String.fromInt (List.length v.board.cells)
                    ++ " tiles, "
                    ++ String.fromInt (List.length v.board.meeples)
                    ++ " meeples on board"
                )
            ]
        ]


viewActions : GameState -> Html Msg
viewActions gs =
    div [ class "actions" ]
        [ button
            [ onClick ClickBotTurn
            , disabled (gs.busy || gs.view.finished)
            ]
            [ text "Bot plays one turn" ]
        , button
            [ onClick ClickRestart
            , style "margin-left" "8px"
            ]
            [ text "New game" ]
        ]


viewEvents : List ScoringEvent -> Html Msg
viewEvents events =
    if List.isEmpty events then
        text ""

    else
        div [ class "events" ]
            [ h2 [] [ text "Last turn scoring" ]
            , ul []
                (List.map
                    (\ev ->
                        li []
                            [ text
                                (featureKindLabel ev.kind
                                    ++ " — "
                                    ++ String.fromInt ev.points
                                    ++ " pts → "
                                    ++ (if List.isEmpty ev.winners then
                                            "no winner"

                                        else
                                            "players "
                                                ++ String.join "," (List.map String.fromInt ev.winners)
                                       )
                                )
                            ]
                    )
                    events
                )
            ]


featureKindLabel : FeatureKind -> String
featureKindLabel k =
    case k of
        FRoad ->
            "Road"

        FCity ->
            "City"

        FMonastery ->
            "Monastery"

        FFarm ->
            "Farm"


viewLegalMoves : GameState -> Html Msg
viewLegalMoves gs =
    let
        n =
            List.length gs.legalMoves

        preview =
            List.take 8 gs.legalMoves
    in
    if n == 0 then
        text ""

    else
        div [ class "legal" ]
            [ h2 [] [ text (String.fromInt n ++ " legal moves (showing first 8)") ]
            , ul []
                (List.map (viewMoveButton gs.busy) preview)
            ]


viewMoveButton : Bool -> GreedyMove -> Html Msg
viewMoveButton busy mv =
    let
        ( x, y ) =
            mv.pos

        meepleStr =
            case mv.meeple of
                Nothing ->
                    "no meeple"

                Just OnMonastery ->
                    "meeple ⛪"

                Just (OnSegment s) ->
                    "meeple " ++ sideLabel s
    in
    li []
        [ button
            [ onClick (ClickPlayMove mv)
            , disabled busy
            ]
            [ text
                ("("
                    ++ String.fromInt x
                    ++ ", "
                    ++ String.fromInt y
                    ++ ") rot "
                    ++ String.fromInt mv.rotation
                    ++ " — "
                    ++ meepleStr
                )
            ]
        ]


sideLabel : Side -> String
sideLabel s =
    case s of
        North ->
            "N"

        East ->
            "E"

        South ->
            "S"

        West ->
            "W"



-- MAIN


main : Program () Model Msg
main =
    Browser.element
        { init = init
        , update = update
        , view = view
        , subscriptions = \_ -> Sub.none
        }

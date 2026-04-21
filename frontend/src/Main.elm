module Main exposing (main)

import Browser
import Html exposing (Html, button, div, h1, h2, li, p, text, ul)
import Html.Attributes exposing (class, disabled, style)
import Html.Events exposing (onClick)
import Http
import Json.Decode as D exposing (Decoder)
import Json.Encode as E
import Set exposing (Set)
import Svg exposing (Svg)
import Svg.Attributes as SA
import Svg.Events as SE



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
    , selectedPos : Maybe Pos
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
    | ClickPos Pos
    | ClickDeselect


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
                , selectedPos = Nothing
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
                    , selectedPos = Nothing
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

        ( ClickPos pos, Playing gs ) ->
            ( Playing { gs | selectedPos = Just pos }, Cmd.none )

        ( ClickDeselect, Playing gs ) ->
            ( Playing { gs | selectedPos = Nothing }, Cmd.none )

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
        , viewBoard gs
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
                , viewDrawSvg spec
                , p [ style "font-family" "monospace", style "font-size" "0.85em" ]
                    [ text (describeTile spec) ]
                ]


viewDrawSvg : TileSpec -> Svg Msg
viewDrawSvg spec =
    let
        pt =
            { spec = spec, rotation = 0 }
    in
    Svg.svg
        [ SA.viewBox "-4 -4 88 88"
        , SA.width "100"
        , SA.style "display:block"
        ]
        (renderTile pt False)


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


viewBoard : GameState -> Html Msg
viewBoard gs =
    let
        v =
            gs.view
    in
    div [ class "board" ]
        [ h2 []
            [ text
                ("Board — "
                    ++ String.fromInt (List.length v.board.cells)
                    ++ " tiles, "
                    ++ String.fromInt (List.length v.board.meeples)
                    ++ " meeples"
                )
            ]
        , viewBoardSvg v.board gs.legalMoves gs.selectedPos
        ]



-- BOARD SVG


tileSize : Int
tileSize =
    80


viewBoardSvg : BoardView -> List GreedyMove -> Maybe Pos -> Svg Msg
viewBoardSvg board legalMoves selectedPos =
    let
        ghosts =
            uniqueLegalPositions legalMoves

        allXs =
            List.map Tuple.first (List.map .pos board.cells ++ ghosts)

        allYs =
            List.map Tuple.second (List.map .pos board.cells ++ ghosts)

        minX =
            List.minimum allXs |> Maybe.withDefault 0

        maxX =
            List.maximum allXs |> Maybe.withDefault 0

        minY =
            List.minimum allYs |> Maybe.withDefault 0

        maxY =
            List.maximum allYs |> Maybe.withDefault 0

        pad =
            tileSize // 2

        x0 =
            minX * tileSize - pad

        y0 =
            -maxY * tileSize - pad

        w =
            (maxX - minX + 1) * tileSize + 2 * pad

        h =
            (maxY - minY + 1) * tileSize + 2 * pad

        viewBoxStr =
            String.join " " (List.map String.fromInt [ x0, y0, w, h ])
    in
    Svg.svg
        [ SA.viewBox viewBoxStr
        , SA.width "800"
        , SA.style "background:#0f1226;border-radius:6px;display:block;max-width:100%"
        ]
        (svgDefs
            :: List.map (viewCell board.lastPlaced) board.cells
            ++ List.map viewMeeple board.meeples
            ++ List.map (viewLegalGhost selectedPos) ghosts
        )


svgDefs : Svg msg
svgDefs =
    Svg.defs []
        [ Svg.pattern
            [ SA.id "fieldPattern"
            , SA.x "0", SA.y "0"
            , SA.width "16", SA.height "16"
            , SA.patternUnits "userSpaceOnUse"
            ]
            [ Svg.rect [ SA.width "16", SA.height "16", SA.fill "#a8cf7d" ] []
            , Svg.circle [ SA.cx "4", SA.cy "5", SA.r "0.7", SA.fill "#7fb35a", SA.opacity "0.7" ] []
            , Svg.circle [ SA.cx "12", SA.cy "10", SA.r "0.7", SA.fill "#7fb35a", SA.opacity "0.7" ] []
            , Svg.circle [ SA.cx "8", SA.cy "13", SA.r "0.5", SA.fill "#6ba049", SA.opacity "0.6" ] []
            ]
        , Svg.pattern
            [ SA.id "cityPattern"
            , SA.x "0", SA.y "0"
            , SA.width "12", SA.height "12"
            , SA.patternUnits "userSpaceOnUse"
            ]
            [ Svg.rect [ SA.width "12", SA.height "12", SA.fill "#d4b896" ] []
            , Svg.rect [ SA.x "1", SA.y "1", SA.width "4", SA.height "4", SA.fill "#c2a276", SA.opacity "0.5" ] []
            , Svg.rect [ SA.x "7", SA.y "7", SA.width "3", SA.height "3", SA.fill "#c2a276", SA.opacity "0.5" ] []
            ]
        ]


uniqueLegalPositions : List GreedyMove -> List Pos
uniqueLegalPositions moves =
    moves
        |> List.map .pos
        |> Set.fromList
        |> Set.toList


viewLegalGhost : Maybe Pos -> Pos -> Svg Msg
viewLegalGhost selected pos =
    let
        ( x, y ) =
            pos

        tx =
            x * tileSize

        ty =
            -y * tileSize

        isSelected =
            selected == Just pos
    in
    Svg.rect
        [ SA.x (String.fromInt tx)
        , SA.y (String.fromInt ty)
        , SA.width "80"
        , SA.height "80"
        , SA.fill
            (if isSelected then
                "rgba(232,197,71,0.25)"

             else
                "rgba(255,255,255,0.04)"
            )
        , SA.stroke
            (if isSelected then
                "#e8c547"

             else
                "#888"
            )
        , SA.strokeDasharray "4 3"
        , SA.strokeWidth "1.5"
        , SA.style "cursor:pointer"
        , SE.onClick (ClickPos pos)
        ]
        []


viewCell : Maybe Pos -> CellView -> Svg Msg
viewCell lastPlaced cell =
    let
        ( x, y ) =
            cell.pos

        tx =
            x * tileSize

        ty =
            -y * tileSize

        isLast =
            lastPlaced == Just cell.pos
    in
    Svg.g
        [ SA.transform ("translate(" ++ String.fromInt tx ++ " " ++ String.fromInt ty ++ ")") ]
        (renderTile cell.tile isLast)


renderTile : PlacedTile -> Bool -> List (Svg Msg)
renderTile pt isLast =
    let
        bg =
            Svg.rect
                [ SA.x "0", SA.y "0", SA.width "80", SA.height "80"
                , SA.fill "url(#fieldPattern)"
                ]
                []

        cities =
            List.filterMap (cityShapeFor pt) [ North, East, South, West ]

        roads =
            List.filterMap (roadShapeFor pt) [ North, East, South, West ]

        monasterySvg =
            if pt.spec.monastery then
                [ monasteryShape ]

            else
                []

        shieldSvg =
            if pt.spec.shield then
                [ shieldShape ]

            else
                []

        border =
            Svg.rect
                [ SA.x "0"
                , SA.y "0"
                , SA.width "80"
                , SA.height "80"
                , SA.fill "none"
                , SA.stroke
                    (if isLast then
                        "#e8c547"

                     else
                        "#3a2f1a"
                    )
                , SA.strokeWidth
                    (if isLast then
                        "3"

                     else
                        "1"
                    )
                ]
                []
    in
    [ bg ] ++ cities ++ roads ++ monasterySvg ++ shieldSvg ++ [ border ]


cityShapeFor : PlacedTile -> Side -> Maybe (Svg Msg)
cityShapeFor pt side =
    if effectiveEdge pt side == City then
        Just
            (Svg.g []
                [ Svg.polygon
                    [ SA.points (cityPolygonPoints side)
                    , SA.fill "url(#cityPattern)"
                    , SA.stroke "#5a3e1f"
                    , SA.strokeWidth "1.5"
                    , SA.strokeLinejoin "round"
                    ]
                    []
                ]
            )

    else
        Nothing


cityPolygonPoints : Side -> String
cityPolygonPoints side =
    case side of
        North ->
            "0,0 80,0 40,40"

        East ->
            "80,0 80,80 40,40"

        South ->
            "0,80 80,80 40,40"

        West ->
            "0,0 0,80 40,40"


roadShapeFor : PlacedTile -> Side -> Maybe (Svg Msg)
roadShapeFor pt side =
    if effectiveEdge pt side == Road then
        let
            ( x1, y1 ) =
                edgeMidpoint side

            sx =
                String.fromInt x1

            sy =
                String.fromInt y1
        in
        Just
            (Svg.g []
                [ Svg.line
                    [ SA.x1 sx, SA.y1 sy, SA.x2 "40", SA.y2 "40"
                    , SA.stroke "#8b6f47"
                    , SA.strokeWidth "11"
                    , SA.strokeLinecap "butt"
                    ]
                    []
                , Svg.line
                    [ SA.x1 sx, SA.y1 sy, SA.x2 "40", SA.y2 "40"
                    , SA.stroke "#cdb084"
                    , SA.strokeWidth "9"
                    , SA.strokeLinecap "butt"
                    ]
                    []
                , Svg.line
                    [ SA.x1 sx, SA.y1 sy, SA.x2 "40", SA.y2 "40"
                    , SA.stroke "#fff"
                    , SA.strokeWidth "1"
                    , SA.strokeDasharray "3 3"
                    , SA.opacity "0.7"
                    ]
                    []
                ]
            )

    else
        Nothing


edgeMidpoint : Side -> ( Int, Int )
edgeMidpoint side =
    case side of
        North ->
            ( 40, 0 )

        East ->
            ( 80, 40 )

        South ->
            ( 40, 80 )

        West ->
            ( 0, 40 )


monasteryShape : Svg Msg
monasteryShape =
    Svg.g []
        [ -- nave (church body)
          Svg.rect
            [ SA.x "30", SA.y "40", SA.width "20", SA.height "16"
            , SA.fill "#f4ead0", SA.stroke "#3a2f1a", SA.strokeWidth "1"
            ]
            []
        , -- nave roof
          Svg.polygon
            [ SA.points "28,40 40,30 52,40"
            , SA.fill "#a83c2c", SA.stroke "#3a2f1a", SA.strokeWidth "1"
            , SA.strokeLinejoin "round"
            ]
            []
        , -- steeple
          Svg.rect
            [ SA.x "37", SA.y "20", SA.width "6", SA.height "12"
            , SA.fill "#f4ead0", SA.stroke "#3a2f1a", SA.strokeWidth "1"
            ]
            []
        , -- steeple roof
          Svg.polygon
            [ SA.points "35,22 40,14 45,22"
            , SA.fill "#a83c2c", SA.stroke "#3a2f1a", SA.strokeWidth "1"
            , SA.strokeLinejoin "round"
            ]
            []
        , -- cross on top
          Svg.line
            [ SA.x1 "40", SA.y1 "10", SA.x2 "40", SA.y2 "16"
            , SA.stroke "#3a2f1a", SA.strokeWidth "1.2"
            ]
            []
        , Svg.line
            [ SA.x1 "37", SA.y1 "12", SA.x2 "43", SA.y2 "12"
            , SA.stroke "#3a2f1a", SA.strokeWidth "1.2"
            ]
            []
        , -- door
          Svg.rect
            [ SA.x "37", SA.y "47", SA.width "6", SA.height "9"
            , SA.fill "#5a3e1f"
            ]
            []
        ]


shieldShape : Svg Msg
shieldShape =
    -- Heraldic shield in NE corner: a rounded-top rectangle that tapers to a point.
    Svg.g []
        [ Svg.path
            [ SA.d "M56,8 L70,8 L70,18 Q70,24 63,26 Q56,24 56,18 Z"
            , SA.fill "#c0392b"
            , SA.stroke "#3a1a0e"
            , SA.strokeWidth "1"
            , SA.strokeLinejoin "round"
            ]
            []
        , -- subtle highlight
          Svg.line
            [ SA.x1 "59", SA.y1 "11", SA.x2 "67", SA.y2 "11"
            , SA.stroke "#e8634f", SA.strokeWidth "1.2", SA.opacity "0.6"
            ]
            []
        ]


viewMeeple : ActiveMeeple -> Svg Msg
viewMeeple m =
    let
        ( x, y ) =
            m.pos

        tx =
            x * tileSize

        ty =
            -y * tileSize

        ( mx, my ) =
            meeplePosition m.on

        color =
            playerColor m.owner

        cx =
            tx + mx

        cy =
            ty + my
    in
    Svg.g
        [ SA.transform
            ("translate(" ++ String.fromInt cx ++ " " ++ String.fromInt cy ++ ")")
        ]
        (meepleSilhouette color)


meepleSilhouette : String -> List (Svg Msg)
meepleSilhouette color =
    [ Svg.circle
        [ SA.cx "0", SA.cy "-7", SA.r "3.5"
        , SA.fill color, SA.stroke "#000", SA.strokeWidth "0.7"
        ]
        []
    , Svg.path
        [ SA.d "M-2,-3 L2,-3 L7,0 L7,4 L4,4 L2,9 L1,9 L0,5 L-1,9 L-2,9 L-4,4 L-7,4 L-7,0 Z"
        , SA.fill color
        , SA.stroke "#000"
        , SA.strokeWidth "0.7"
        , SA.strokeLinejoin "round"
        ]
        []
    ]


meeplePosition : MeepleChoice -> ( Int, Int )
meeplePosition c =
    case c of
        OnMonastery ->
            ( 40, 50 )

        OnSegment side ->
            case side of
                North ->
                    ( 40, 22 )

                East ->
                    ( 58, 40 )

                South ->
                    ( 40, 58 )

                West ->
                    ( 22, 40 )


playerColor : Int -> String
playerColor i =
    case i of
        0 ->
            "#e74c3c"

        1 ->
            "#3498db"

        2 ->
            "#2ecc71"

        3 ->
            "#f1c40f"

        _ ->
            "#9b59b6"


effectiveEdge : PlacedTile -> Side -> EdgeKind
effectiveEdge pt side =
    let
        canonical =
            (sideToInt side - pt.rotation + 4) |> modBy 4
    in
    pt.spec.edges
        |> List.drop canonical
        |> List.head
        |> Maybe.withDefault Field


sideToInt : Side -> Int
sideToInt s =
    case s of
        North ->
            0

        East ->
            1

        South ->
            2

        West ->
            3


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
        filtered =
            case gs.selectedPos of
                Nothing ->
                    gs.legalMoves

                Just sp ->
                    List.filter (\m -> m.pos == sp) gs.legalMoves

        n =
            List.length filtered

        ( shown, headerText ) =
            case gs.selectedPos of
                Nothing ->
                    ( List.take 8 filtered
                    , String.fromInt (List.length gs.legalMoves)
                        ++ " legal moves — click a yellow ghost on the board to filter (or pick from first 8 below)"
                    )

                Just ( x, y ) ->
                    ( filtered
                    , String.fromInt n
                        ++ " moves at ("
                        ++ String.fromInt x
                        ++ ", "
                        ++ String.fromInt y
                        ++ ")"
                    )

        controls =
            case gs.selectedPos of
                Just _ ->
                    [ button [ onClick ClickDeselect ] [ text "Clear selection" ] ]

                Nothing ->
                    []
    in
    if List.isEmpty gs.legalMoves then
        text ""

    else
        div [ class "legal" ]
            ([ h2 [] [ text headerText ] ]
                ++ controls
                ++ [ ul [] (List.map (viewMoveButton gs.busy) shown) ]
            )


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
